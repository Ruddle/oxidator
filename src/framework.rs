use log::info;
use winit::event::{Event, WindowEvent};

extern crate nalgebra as na;
use na::{Matrix4, Rotation3, Vector3};

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
    use std::mem::size_of;
    use std::slice::from_raw_parts;

    unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>()) }
}

#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

pub fn load_glsl(code: &str, stage: ShaderStage) -> Vec<u32> {
    let ty = match stage {
        ShaderStage::Vertex => glsl_to_spirv::ShaderType::Vertex,
        ShaderStage::Fragment => glsl_to_spirv::ShaderType::Fragment,
        ShaderStage::Compute => glsl_to_spirv::ShaderType::Compute,
    };

    wgpu::read_spirv(glsl_to_spirv::compile(&code, ty).unwrap()).unwrap()
}

pub trait Example: 'static + Sized {
    fn init(
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &mut wgpu::Device,
        window: &winit::window::Window,
        hidpi_factor: f64,
    ) -> (Self, Option<wgpu::CommandBuffer>);
    fn resize(
        &mut self,
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
    ) -> Option<wgpu::CommandBuffer>;
    fn update(&mut self, event: &Event<()>, window: &winit::window::Window);
    fn render(
        &mut self,
        frame: &wgpu::SwapChainOutput,
        device: &mut wgpu::Device,
        window: &winit::window::Window,
    ) -> wgpu::CommandBuffer;
}

pub fn run<E: Example>(title: &str) {
    use winit::{
        event,
        event_loop::{ControlFlow, EventLoop},
    };

    env_logger::init();
    let event_loop = EventLoop::new();
    info!("Initializing the window...");

    let (window, instance, hidpi_factor, size, surface) = {
        let instance = wgpu::Instance::new();

        let window = winit::window::Window::new(&event_loop).unwrap();
        window.set_inner_size(winit::dpi::LogicalSize {
            width: 1280.0,
            height: 720.0,
        });
        window.set_title(title);
        let hidpi_factor = window.hidpi_factor();
        let size = window.inner_size().to_physical(hidpi_factor);

        use raw_window_handle::HasRawWindowHandle as _;
        let surface = instance.create_surface(window.raw_window_handle());

        (window, instance, hidpi_factor, size, surface)
    };

    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
    });

    let mut device: wgpu::Device = adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    });

    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: size.width.round() as u32,
        height: size.height.round() as u32,
        present_mode: wgpu::PresentMode::NoVsync,
    };
    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    info!("Initializing the example...");
    let (mut example, init_command_buf) = E::init(&sc_desc, &mut device, &window, hidpi_factor);
    if let Some(command_buf) = init_command_buf {
        device.get_queue().submit(&[command_buf]);
    }

    info!("Entering render loop...");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };

        example.update(&event, &window);
        match event {
            event::Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                let physical = size.to_physical(hidpi_factor);
                info!("Resizing to {:?}", physical);
                sc_desc.width = physical.width.round() as u32;
                sc_desc.height = physical.height.round() as u32;
                swap_chain = device.create_swap_chain(&surface, &sc_desc);
                let command_buf = example.resize(&sc_desc, &device);
                if let Some(command_buf) = command_buf {
                    device.get_queue().submit(&[command_buf]);
                }
            }
            event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(event::VirtualKeyCode::Escape),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            },
            event::Event::EventsCleared => {
                let frame = swap_chain.get_next_texture();
                let command_buf = example.render(&frame, &mut device, &window);
                device.get_queue().submit(&[command_buf]);
            }
            _ => (),
        }
    });
}

// This allows treating the framework as a standalone example,
// thus avoiding listing the example names in `Cargo.toml`.
#[allow(dead_code)]
fn main() {}
