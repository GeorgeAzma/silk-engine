pub type Event = winit::event::WindowEvent;
pub type Key = winit::keyboard::KeyCode;
pub type Mouse = winit::event::MouseButton;

pub struct Input {
    mouse: [bool; 5],
    mouse_old: [bool; 5],
    mouse_x: f32,
    mouse_y: f32,
    mouse_scroll: f32,
    mouse_press_x: [f32; 5],
    mouse_press_y: [f32; 5],
    key: [bool; 194],
    key_old: [bool; 194],
    focus: bool,
    focus_old: bool,
}

impl Input {
    pub fn new() -> Self {
        Self {
            mouse: [false; 5],
            mouse_old: [false; 5],
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_scroll: 0.0,
            mouse_press_x: [0.0; 5],
            mouse_press_y: [0.0; 5],
            key: [false; 194],
            key_old: [false; 194],
            focus: true,
            focus_old: false,
        }
    }

    pub fn event(&mut self, event: &Event, width: u32, height: u32) {
        match event {
            Event::CursorMoved {
                device_id: _,
                position,
            } => {
                self.mouse_x = position.x as f32 / width as f32 * 2.0 - 1.0;
                self.mouse_y = 1.0 - position.y as f32 / height as f32 * 2.0;
            }
            Event::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                self.mouse[Self::mouse_idx(*button)] = state.is_pressed();
                if state.is_pressed() {
                    self.mouse_press_x[Self::mouse_idx(*button)] = self.mouse_x;
                    self.mouse_press_y[Self::mouse_idx(*button)] = self.mouse_y;
                }
            }
            Event::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                use winit::event::MouseScrollDelta;
                match delta {
                    MouseScrollDelta::LineDelta(_, y) => self.mouse_scroll = *y,
                    MouseScrollDelta::PixelDelta(p) => {
                        self.mouse_scroll = p.y as f32 / height as f32
                    }
                }
            }
            Event::Touch(touch) => {
                self.mouse_x = touch.location.x as f32;
                self.mouse_y = touch.location.y as f32;
                use winit::event::TouchPhase;
                match touch.phase {
                    TouchPhase::Started | TouchPhase::Moved => self.mouse[0] = true,
                    TouchPhase::Ended | TouchPhase::Cancelled => self.mouse[0] = false,
                }
            }
            Event::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let winit::keyboard::PhysicalKey::Code(key) = event.physical_key {
                    self.key[key as usize] = event.state.is_pressed();
                }
            }
            Event::Focused(focus) => {
                self.focus = *focus;
                if !self.focus {
                    self.reset();
                }
            }
            _ => {}
        }
    }

    pub fn reset(&mut self) {
        self.mouse_scroll = 0.0;
        self.mouse_old = self.mouse;
        self.key_old = self.key;
        self.focus_old = self.focus;
    }

    pub fn mouse_x(&self) -> f32 {
        self.mouse_x
    }

    pub fn mouse_y(&self) -> f32 {
        self.mouse_y
    }

    pub fn mouse_scroll(&self) -> f32 {
        self.mouse_scroll
    }

    pub fn mouse_pressed(&self, m: Mouse) -> bool {
        !self.mouse_old[Self::mouse_idx(m)] && self.mouse[Self::mouse_idx(m)]
    }

    pub fn mouse_released(&self, m: Mouse) -> bool {
        self.mouse_old[Self::mouse_idx(m)] && !self.mouse[Self::mouse_idx(m)]
    }

    pub fn mouse_down(&self, m: Mouse) -> bool {
        self.mouse[Self::mouse_idx(m)]
    }

    pub fn mouse_press_x(&self, m: Mouse) -> f32 {
        self.mouse_press_x[Self::mouse_idx(m)]
    }

    pub fn mouse_press_y(&self, m: Mouse) -> f32 {
        self.mouse_press_y[Self::mouse_idx(m)]
    }

    pub fn mouse_drag_x(&self, m: Mouse) -> f32 {
        self.mouse_x - self.mouse_press_x[Self::mouse_idx(m)]
    }

    pub fn mouse_drag_y(&self, m: Mouse) -> f32 {
        self.mouse_y - self.mouse_press_y[Self::mouse_idx(m)]
    }

    pub fn key_pressed(&self, k: Key) -> bool {
        !self.key_old[k as usize] && self.key[k as usize]
    }

    pub fn key_released(&self, k: Key) -> bool {
        self.key_old[k as usize] && !self.key[k as usize]
    }

    pub fn key_down(&self, k: Key) -> bool {
        self.key[k as usize]
    }

    pub fn focused(&self) -> bool {
        !self.focus_old && self.focus
    }

    fn mouse_idx(mouse: Mouse) -> usize {
        match mouse {
            Mouse::Left => 0,
            Mouse::Right => 1,
            Mouse::Middle => 2,
            Mouse::Back => 3,
            Mouse::Forward => 4,
            Mouse::Other(..) => 0,
        }
    }
}
