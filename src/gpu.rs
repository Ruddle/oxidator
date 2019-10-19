use wgpu::SwapChain;

pub struct WgpuState {
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub device: wgpu::Device,
    pub window: winit::window::Window,
    pub hidpi_factor: f64,
    pub swap_chain: SwapChain,
    pub surface: wgpu::Surface,
}

impl WgpuState {
    pub fn new(window: winit::window::Window) -> Self {
        let (instance, hidpi_factor, size, surface) = {
            let instance = wgpu::Instance::new();

            window.set_inner_size(winit::dpi::LogicalSize {
                width: 1280.0,
                height: 720.0,
            });
            window.set_title("Oxidator");
            let hidpi_factor = window.hidpi_factor();
            let size = window.inner_size().to_physical(hidpi_factor);

            use raw_window_handle::HasRawWindowHandle as _;
            let surface = instance.create_surface(window.raw_window_handle());

            (instance, hidpi_factor, size, surface)
        };

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
        });

        let mut device: wgpu::Device = adapter.request_device(&wgpu::DeviceDescriptor {
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

        WgpuState {
            sc_desc,
            device,
            window,
            hidpi_factor,
            swap_chain,
            surface,
        }
    }
}
