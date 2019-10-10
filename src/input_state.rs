use std::collections::HashSet;

pub struct InputState {
    pub key_pressed: HashSet<winit::event::VirtualKeyCode>,
    pub last_scroll: f32,
    pub fps: u64,
    pub debug_i1: i32,
    pub cursor_pos: (u32, u32),
}

impl InputState {
    pub fn new() -> Self {
        InputState {
            key_pressed: HashSet::new(),
            last_scroll: 0.0,
            fps: 144,
            debug_i1: 1,
            cursor_pos: (0, 0),
        }
    }
}
