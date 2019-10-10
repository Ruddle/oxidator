mod app;
mod camera;
mod fake_texels;
mod game_state;
mod glsl_compiler;
mod heightmap;
mod heightmap_gpu;
mod input_state;
mod model;
mod model_gpu;
extern crate nalgebra as na;
extern crate shaderc;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new();
    let mut app = app::App::init(&event_loop);
    let mut counter = 0;
    event_loop.run(move |event, _, control_flow| {
        counter += 1;
        println!("{:?}", event);
        app.update(&event, control_flow);
    });
}
