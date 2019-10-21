mod camera;
mod client;
mod fake_texels;
mod frame;
mod frame_server;
mod game_state;
mod glsl_compiler;
mod gpu;
mod group_behavior;
mod heightmap;
mod heightmap_editor;
mod heightmap_gpu;
mod input_state;
mod mobile;
mod model;
mod model_gpu;
mod phy_state;
mod post_fx;
mod post_fxaa;

mod utils;

#[macro_use]
extern crate load_file;
extern crate byteorder;
extern crate crossbeam_channel;
extern crate nalgebra as na;
extern crate shaderc;
#[macro_use]
extern crate typename;
extern crate base_62;

use winit::event::Event;
use winit::event_loop::ControlFlow;

#[derive(Debug)]
pub enum ToClient {
    EventMessage { event: Event<()> },
    MapReadAsyncMessage { vec: Vec<f32>, usage: String },
    Render,
    NewFrame(frame::Frame),
}

pub enum EventLoopMsg {
    Stop,
}

//mod test;
fn main() {
    //    test::main();
    println!("dir: {:?}", std::env::current_dir().unwrap());
    env_logger::init();
    log::trace!("Starting main system");

    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();

    use crossbeam_channel::unbounded;
    let (s_to_client, r_to_client) = unbounded::<ToClient>();
    let s_to_client_for_root_manager = s_to_client.clone();
    let s_to_client_for_event_loop = s_to_client.clone();

    let (s_to_event_loop, r_to_event_loop) = unbounded::<EventLoopMsg>();
    let (s_from_client, r_from_client) = unbounded::<client::FromClient>();
    let (s_to_frame_server, r_to_frame_server) = unbounded::<frame_server::ToFrameServer>();
    let (s_from_frame_server, r_from_frame_server) = unbounded::<frame_server::FromFrameServer>();

    //Client
    std::thread::spawn(move || {
        let _ = s_to_client.send(ToClient::Render);
        let mut client = client::App::new(window, s_to_client, r_to_client, s_to_event_loop,s_from_client);
        loop {
            client.receive();
        }
    });

    //Frame server
    std::thread::spawn(move || loop {
        match r_to_frame_server.recv() {
            Ok(frame_server::ToFrameServer::AskNextFrameMsg { old_frame }) => {
                let next_frame = frame_server::next_frame(old_frame);
                let _ =
                    s_from_frame_server.send(frame_server::FromFrameServer::NewFrame(next_frame));
            }
            _ => {}
        }
    });

    //Root manager
    std::thread::spawn(move || {
        let frame0 = frame::Frame::new();
        let _ = s_to_frame_server.send(frame_server::ToFrameServer::AskNextFrameMsg {
            old_frame: frame0.clone(),
        });

        let _ = s_to_client_for_root_manager.send(ToClient::NewFrame(frame0));
        loop {
            //Waiting before receiving new partial frames
            let wait_duration = std::time::Duration::from_millis(100);
            std::thread::sleep(wait_duration);

            //Receiving Partial new frames
            let mut new_partial_frame = match r_from_frame_server.recv() {
                Ok(frame_server::FromFrameServer::NewFrame(new_frame)) => new_frame,
                _ => panic!("No frame from frame_server"),
            };

            let client_events: Vec<_> = r_from_client
                .try_iter()
                .map(|client::FromClient::Event(event)| event)
                .collect();

            new_partial_frame.events.extend(client_events);
            //Frame is now complete and ready to be sent
            let full_frame = new_partial_frame;

            //Sending frame
            let _ = s_to_client_for_root_manager.send(ToClient::NewFrame(full_frame.clone()));
            let _ = s_to_frame_server.send(frame_server::ToFrameServer::AskNextFrameMsg {
                old_frame: full_frame,
            });
        }
    });

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { .. } => {
            s_to_client_for_event_loop
                .send(ToClient::EventMessage { event })
                .unwrap();
        }
        Event::EventsCleared => {
            match r_to_event_loop.try_recv() {
                Ok(EventLoopMsg::Stop) => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            }
            std::thread::sleep(std::time::Duration::from_millis(4));
        }
        _ => {}
    });
}
