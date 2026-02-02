use std::collections::HashMap;

use winit::event::{Force, WindowEvent};

pub type Key = winit::keyboard::KeyCode;
pub type Mouse = winit::event::MouseButton;

pub struct Input {
    /// press state for each mouse button
    mouse: [bool; 5],
    mouse_old: [bool; 5],
    /// [-1; 1] normalized mouse/touch/stylus X coordinate
    mouse_x: f32,
    /// [-1; 1] normalized mouse/touch/stylus Y coordinate
    mouse_y: f32,
    mouse_scroll: f32,
    // drag X location of each mouse button
    mouse_press_x: [f32; 5],
    // drag Y location of each mouse button
    mouse_press_y: [f32; 5],
    /// unnormalized or [0; 1] normalized pressure of a single touch or a stylus.
    pressure: f32,
    /// maximum possible pressure of a single touch or a stylus
    max_pressure: f32,
    /// stylus angle, 0 = parallel to surface (fully tilted), pi/2 = perpendicular to surface (held upright)
    angle: f32,
    key: [bool; 194],
    key_old: [bool; 194],
    /// active touches at X,Y with force/pressure
    active_touches: HashMap<u64, (f32, f32, Force)>,
    /// window focus state
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
            pressure: 0.0,
            max_pressure: 0.0,
            angle: 0.0,
            key: [false; 194],
            key_old: [false; 194],
            active_touches: HashMap::default(),
            focus: true,
            focus_old: false,
        }
    }

    pub fn event(&mut self, event: &WindowEvent, width: u32, height: u32) {
        match event {
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                if width > 0 {
                    self.mouse_x = position.x as f32 / width as f32 * 2.0 - 1.0;
                }
                if height > 0 {
                    self.mouse_y = 1.0 - position.y as f32 / height as f32 * 2.0;
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                let idx = Self::mouse_idx(*button);
                if state.is_pressed() {
                    self.pressure = 1.0;
                    self.max_pressure = 1.0;
                    self.angle = 0.0;
                    self.mouse[idx] = true;
                    self.mouse_press_x[idx] = self.mouse_x;
                    self.mouse_press_y[idx] = self.mouse_y;
                } else {
                    self.mouse[idx] = false;
                    self.pressure = 0.0;
                }
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                use winit::event::MouseScrollDelta;
                match delta {
                    MouseScrollDelta::LineDelta(_, y) => self.mouse_scroll = *y,
                    MouseScrollDelta::PixelDelta(p) => {
                        if height > 0 {
                            self.mouse_scroll = p.y as f32 / height as f32
                        }
                    }
                }
            }
            WindowEvent::Touch(touch) => {
                let x = touch.location.x as f32;
                let y = touch.location.y as f32;
                self.mouse_x = x;
                self.mouse_y = y;
                use winit::event::TouchPhase;
                match touch.phase {
                    TouchPhase::Started | TouchPhase::Moved => {
                        let idx = Self::mouse_idx(Mouse::Left);
                        if let Some(force) = touch.force {
                            match force {
                                Force::Calibrated {
                                    force: pressure,
                                    max_possible_force,
                                    altitude_angle,
                                } => {
                                    self.pressure = pressure as f32;
                                    self.max_pressure = max_possible_force as f32;
                                    self.angle = altitude_angle.unwrap_or(0.0) as f32;
                                    self.mouse[idx] = true;
                                    self.mouse_press_x[idx] = x;
                                    self.mouse_press_y[idx] = y;
                                    self.active_touches.insert(touch.id, (x, y, force));
                                }
                                Force::Normalized(pressure) => {
                                    self.pressure = pressure as f32;
                                    self.max_pressure = 1.0;
                                    self.angle = 0.0;
                                    self.mouse[idx] = true;
                                    self.mouse_press_x[idx] = x;
                                    self.mouse_press_y[idx] = y;
                                    let force = touch.force.unwrap();
                                    self.active_touches.insert(touch.id, (x, y, force));
                                }
                            }
                        } else {
                            self.pressure = 0.0;
                            self.max_pressure = 1.0;
                        }
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        let idx = Self::mouse_idx(Mouse::Left);
                        self.mouse[idx] = false;
                        self.pressure = 0.0;
                        self.active_touches.remove(&touch.id);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let winit::keyboard::PhysicalKey::Code(key) = event.physical_key {
                    self.key[key as usize] = event.state.is_pressed();
                }
            }
            WindowEvent::Focused(focus) => {
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

    pub fn touch(&self, touch_id: u64) -> Option<(f32, f32, Force)> {
        self.active_touches.get(&touch_id).copied()
    }

    pub fn touched(&self, touch_id: u64) -> bool {
        self.active_touches.contains_key(&touch_id)
    }

    pub fn touch_x(&self, touch_id: u64) -> f32 {
        self.active_touches
            .get(&touch_id)
            .map_or(0.0, |&(x, _, _)| x)
    }

    pub fn touch_y(&self, touch_id: u64) -> f32 {
        self.active_touches
            .get(&touch_id)
            .map_or(0.0, |&(_, y, _)| y)
    }

    pub fn touch_force(&self, touch_id: u64) -> Force {
        self.active_touches
            .get(&touch_id)
            .map_or(Force::Normalized(0.0), |&(_, _, force)| force)
    }

    pub fn touch_pressure(&self, touch_id: u64) -> f32 {
        match self.touch_force(touch_id) {
            Force::Calibrated {
                force,
                max_possible_force: _,
                altitude_angle: _,
            } => force as f32,
            Force::Normalized(force) => force as f32,
        }
    }

    pub fn touch_max_pressure(&self, touch_id: u64) -> f32 {
        match self.touch_force(touch_id) {
            Force::Calibrated {
                force: _,
                max_possible_force,
                altitude_angle: _,
            } => max_possible_force as f32,
            Force::Normalized(_force) => 1.0,
        }
    }

    pub fn touch_angle(&self, touch_id: u64) -> f32 {
        match self.touch_force(touch_id) {
            Force::Calibrated {
                force: _,
                max_possible_force: _,
                altitude_angle,
            } => altitude_angle.unwrap_or(0.0) as f32,
            Force::Normalized(_force) => 0.0,
        }
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
