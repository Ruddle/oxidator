use std::collections::HashSet;

pub enum Drag {
    None,
    Start { x0: u32, y0: u32 },
    Dragging { x0: u32, y0: u32, x1: u32, y1: u32 },
    End { x0: u32, y0: u32, x1: u32, y1: u32 },
}

pub struct InputState {
    pub key_pressed: HashSet<winit::event::VirtualKeyCode>,
    pub mouse_pressed: HashSet<winit::event::MouseButton>,

    pub key_trigger: HashSet<winit::event::VirtualKeyCode>,
    pub mouse_trigger: HashSet<winit::event::MouseButton>,

    pub key_release: HashSet<winit::event::VirtualKeyCode>,
    pub mouse_release: HashSet<winit::event::MouseButton>,

    pub drag: Drag,

    pub last_scroll: f32,
    pub fps: u64,
    pub debug_i1: i32,
    pub cursor_pos: (u32, u32),
}

impl InputState {
    pub fn new() -> Self {
        InputState {
            key_pressed: HashSet::new(),
            mouse_pressed: HashSet::new(),
            key_trigger: HashSet::new(),
            mouse_trigger: HashSet::new(),
            key_release: HashSet::new(),
            mouse_release: HashSet::new(),
            last_scroll: 0.0,
            fps: 144,
            debug_i1: 1,
            cursor_pos: (0, 0),
            drag: Drag::None,
        }
    }

    pub fn update(&mut self) {
        self.key_trigger.clear();
        self.mouse_trigger.clear();
        self.mouse_release.clear();
        self.key_release.clear();
        if let Drag::End { .. } = self.drag {
            self.drag = Drag::None;
        }
    }
}
