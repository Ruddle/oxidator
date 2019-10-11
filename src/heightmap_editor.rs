use imgui::*;

pub struct State {
    pub pen_radius: u32,
    pub pen_strength: f32,
}

impl State {
    pub fn new() -> Self {
        State {
            pen_radius: 50,
            pen_strength: 1.0,
        }
    }

    pub fn draw(&mut self, ui: &Ui) {
        let pen_radius = &mut self.pen_radius;
        let pen_strength = &mut self.pen_strength;
        let edit_height_window = imgui::Window::new(im_str!("Heightmap editor"));
        edit_height_window
            .size([400.0, 200.0], imgui::Condition::FirstUseEver)
            .position([3.0, 206.0], imgui::Condition::FirstUseEver)
            .build(&ui, || {
                imgui::Slider::new(im_str!("pen radius"), 1..=1000).build(&ui, pen_radius);
                imgui::Slider::new(im_str!("pen strength"), 0.0..=10.0).build(&ui, pen_strength);
            });
    }
}
