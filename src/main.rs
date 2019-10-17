mod app;
mod camera;
mod fake_texels;
mod game_state;
mod glsl_compiler;
mod heightmap;
mod heightmap_editor;
mod heightmap_gpu;
mod input_state;
mod mobile;
mod model;
mod model_gpu;
mod phy_state;
mod post_fx;

extern crate crossbeam_channel;
extern crate nalgebra as na;
extern crate shaderc;

use winit::event::Event;
use winit::event_loop::ControlFlow;

#[derive(Debug)]
pub enum AppMsg {
    EventMessage { event: Event<()> },
    MapReadAsyncMessage { vec: Vec<f32> },
    Render,
}

pub enum EventLoopMsg {
    Stop,
}

//mod test;
fn main() {
    //    test::main();

    env_logger::init();
    log::trace!("Starting actix system");

    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::Window::new(&event_loop).unwrap();

    use crossbeam_channel::unbounded;
    let (s_app, r_app) = unbounded::<AppMsg>();
    let s_app_for_event_loop = s_app.clone();

    let (s_event_loop, r_event_loop) = unbounded::<EventLoopMsg>();

    std::thread::spawn(move || {
        let _ = s_app.send(AppMsg::Render);
        let mut app = app::App::new(window, s_app, r_app, s_event_loop);
        loop {
            app.receive();
        }
    });

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { .. } => {
            s_app_for_event_loop
                .send(AppMsg::EventMessage { event })
                .unwrap();
        }
        Event::EventsCleared => {
            match r_event_loop.try_recv() {
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
