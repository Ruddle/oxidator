use wgpu::SwapChain;

pub struct WgpuState {
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub device: wgpu::Device,
    pub window: winit::window::Window,
    pub hidpi_factor: f64,
    pub swap_chain: SwapChain,
    pub surface: wgpu::Surface,
    pub queue: wgpu::Queue,
}

impl WgpuState {
    pub fn new(window: winit::window::Window) -> Self {
        let (hidpi_factor, size, surface) = {
            let hidpi_factor = window.hidpi_factor();

            window.set_title("Oxidator");
            log::info!("hidpi scaling: {}", hidpi_factor);
            let size = window.inner_size().to_physical(hidpi_factor);
            let surface = wgpu::Surface::create(&window);
            (hidpi_factor, size, surface)
        };

        let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            backends: wgpu::BackendBit::PRIMARY,
        })
        .unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
            limits: wgpu::Limits::default(),
        });

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width.round() as u32,
            height: size.height.round() as u32,
            present_mode: wgpu::PresentMode::NoVsync,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        // let physical_wanted = winit::dpi::PhysicalSize {
        //     width: 1281.0,
        //     height: 720.0,
        // };
        // let logical_wanted = physical_wanted.to_logical(hidpi_factor);
        // window.set_inner_size(logical_wanted);

        WgpuState {
            sc_desc,
            device,
            window,
            hidpi_factor,
            swap_chain,
            surface,
            queue,
        }
    }
}
