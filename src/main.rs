mod app;
mod camera;
mod fake_texels;
mod game_state;
mod glsl_compiler;
mod heightmap_editor;
mod heightmap;
mod heightmap_gpu;
mod input_state;
mod model;
mod model_gpu;
extern crate nalgebra as na;
extern crate shaderc;

use actix::prelude::*;
use actix::Recipient;

use winit::event::Event;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

#[derive(Message)]
struct EventMessage {
    event: Event<()>,
}

#[derive(Message)]
struct WindowMessage {
    window: Window,
}

#[derive(Message)]
struct MapReadAsyncMessage {
    vec: Vec<f32>,
}

pub struct AppActor {
    app: Option<app::App>,
}
impl Actor for AppActor {
    type Context = Context<AppActor>;
    fn started(&mut self, ctx: &mut Self::Context) {}
}
impl Handler<EventMessage> for AppActor {
    type Result = ();
    fn handle(&mut self, msg: EventMessage, ctx: &mut Context<Self>) {
        if let Some(app) = &mut self.app {
            match msg.event {
                Event::EventsCleared => {
                    ctx.notify(EventMessage {
                        event: Event::EventsCleared,
                    });
                }
                _ => {}
            }

            app.update(&msg.event, ctx.address());
        }
    }
}

impl Handler<WindowMessage> for AppActor {
    type Result = ();
    fn handle(&mut self, msg: WindowMessage, ctx: &mut Context<Self>) {
        self.app = Some(app::App::init(msg.window));
    }
}

impl Handler<MapReadAsyncMessage> for AppActor {
    type Result = ();
    fn handle(&mut self, msg: MapReadAsyncMessage, ctx: &mut Context<Self>) {
        if let Some(app) = &mut self.app {
            app.map_read_async_msg(msg.vec)
        }
    }
}

fn main() {
    let system = System::new("system");

    let app = AppActor { app: None }.start();

    std::thread::spawn(move || {
        let event_loop = winit::event_loop::EventLoop::new();
        let window = winit::window::Window::new(&event_loop).unwrap();
        let m = WindowMessage { window };
        app.do_send(m);
        app.do_send(EventMessage {
            event: Event::EventsCleared,
        });

        event_loop.run(move |event, _, control_flow| {
            if let Event::EventsCleared = event {
                std::thread::sleep(std::time::Duration::from_millis(4));
            }
            match event {
                Event::WindowEvent { .. } => {
                    app.do_send(EventMessage { event });
                }
                _ => {}
            }
        });
    });

    system.run();
}
