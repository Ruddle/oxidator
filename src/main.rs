mod client;
mod frame;
mod frame_server;
mod glsl;
mod gpu_obj;
mod heightmap_phy;
mod manager;
mod mobile;
mod model;
mod net_client;
mod net_server;
mod procedural_texels;
mod unit;

mod utils;
extern crate byteorder;
extern crate crossbeam_channel;
extern crate nalgebra as na;
#[cfg(feature = "use_shaderc")]
extern crate shaderc;
#[macro_use]
extern crate typename;
extern crate base_62;
extern crate rayon;
extern crate spin_sleep;
use crossbeam_channel::unbounded;
use spin_sleep::LoopHelper;
use winit::event::Event;
use winit::event_loop::ControlFlow;
#[derive(Debug)]
pub enum ToClient {
    MapReadAsyncMessage { vec: Vec<f32>, usage: String },
    NewFrame(frame::Frame),
    GlobalInfo(manager::GlobalInfo),
}

pub enum EventLoopMsg {
    Stop,
}
use std::env;
fn main() {
    env_logger::init();
    if let Some(x) = env::args().skip(1).next() {
        if x == "compile" {
            glsl::compile_all_glsl();
        }
    } else {
        do_the_thing();
    }
}

fn do_the_thing() {
    let (s_to_frame_server, r_to_frame_server) = unbounded::<frame_server::ToFrameServer>();
    let (s_from_frame_server, r_from_frame_server) = unbounded::<frame_server::FromFrameServer>();

    let frame_server =
        frame_server::FrameServerCache::spawn(r_to_frame_server, s_from_frame_server);

    let (s_from_client_to_manager, r_from_client_to_manager) = unbounded::<client::FromClient>();
    let (s_to_client, r_to_client) = unbounded::<ToClient>();
    let s_to_client_from_manager = s_to_client.clone();
    let manager = manager::Manager::new(
        s_to_client_from_manager,
        s_to_frame_server,
        r_from_frame_server,
        r_from_client_to_manager,
    );

    let (s_to_event_loop, r_to_event_loop) = unbounded::<EventLoopMsg>();
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    let mut client = client::App::new(
        window,
        s_to_client,
        r_to_client,
        s_to_event_loop,
        s_from_client_to_manager,
    );

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { .. } => {
            client.handle_winit_event(&event);
        }
        Event::EventsCleared => match r_to_event_loop.try_recv() {
            Ok(EventLoopMsg::Stop) => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {
                client.receive();
                client.render();
            }
        },
        _ => {}
    });
}
