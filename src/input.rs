
use std::collections::HashMap;

use winit::{dpi::{PhysicalPosition, PhysicalSize}, event, window::Window};



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputID {
    Confirm,
    Cancel,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyState {
    pub pressed: bool,
    pub just_pressed: bool,
    pub just_released: bool,
}

impl KeyState {
    fn new() -> KeyState {
        KeyState {
            pressed: false,
            just_pressed: false,
            just_released: false,
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Input {
    pub key_state: KeyState,
    pub identifier: InputID,
    pub keys: Vec<winit::keyboard::KeyCode>,
}

impl Input {
    fn new(identifier: InputID, keys: Vec<winit::keyboard::KeyCode>) -> Input {
        Input {
            key_state: KeyState::new(),
            identifier,
            keys,
        }
    }
}

pub struct InputHandler<'a> {
    window: &'a Window,

    mouse_x: f32,
    mouse_y: f32,
    screen_width: u32,
    screen_height: u32,
    mouse_sensitivity: f32,

    inputs_received: Vec<event::KeyEvent>,
    inputs: HashMap<InputID, Input>,
}

pub trait InputDriveInterface {
    fn new(window: &Window) -> InputHandler;
    fn activate(&mut self);
    fn deactivate(&mut self);
    fn cursor_moved(&mut self, position: &PhysicalPosition<f64>);
    fn window_resized(&mut self, size: PhysicalSize<u32>);
    fn update(&mut self, dt: f32);
    fn handle_event(&mut self, event: &winit::event::KeyEvent);
}

pub trait InputGetInterface {
    fn get_mouse_x(&self) -> f32;
    fn get_mouse_y(&self) -> f32;
    fn get_key_state(&self, key: InputID) -> &KeyState;
}

impl InputGetInterface for InputHandler<'_> {
    fn get_mouse_x(&self) -> f32 {
        self.mouse_x
    }

    fn get_mouse_y(&self) -> f32 {
        self.mouse_y
    }

    fn get_key_state(&self, key: InputID) -> &KeyState {
        &self.inputs[&key].key_state
    }
}

impl InputHandler<'_> {
    pub fn debug_print_keys(&self) {
        for (key, input) in &self.inputs {
            println!("{:?} {:?}", key, input.key_state);
        }
    }
}

impl<'a> InputDriveInterface for InputHandler<'a> {
    fn new(window: &Window) -> InputHandler {

        let mut inputs = HashMap::new();
        inputs.insert(InputID::Confirm, Input::new(InputID::Confirm, vec![
            winit::keyboard::KeyCode::Space,
            winit::keyboard::KeyCode::Enter,
            winit::keyboard::KeyCode::KeyE,
        ]));
        inputs.insert(InputID::Cancel, Input::new(InputID::Cancel, vec![
            winit::keyboard::KeyCode::Escape,
            winit::keyboard::KeyCode::Backspace,
            winit::keyboard::KeyCode::KeyQ,
        ]));
        inputs.insert(InputID::Up, Input::new(InputID::Up, vec![
            winit::keyboard::KeyCode::KeyW,
            winit::keyboard::KeyCode::ArrowUp,
        ]));
        inputs.insert(InputID::Down, Input::new(InputID::Down, vec![
            winit::keyboard::KeyCode::KeyS,
            winit::keyboard::KeyCode::ArrowDown,
        ]));
        inputs.insert(InputID::Left, Input::new(InputID::Left, vec![
            winit::keyboard::KeyCode::KeyA,
            winit::keyboard::KeyCode::ArrowLeft,
        ]));
        inputs.insert(InputID::Right, Input::new(InputID::Right, vec![
            winit::keyboard::KeyCode::KeyD,
            winit::keyboard::KeyCode::ArrowRight,
        ]));

        InputHandler {
            window,
            mouse_x: 0.0,
            mouse_y: 0.0,
            screen_width: window.inner_size().width,
            screen_height: window.inner_size().height,
            mouse_sensitivity: 40.0,
            inputs,
            inputs_received: vec![],
        }
    }

    fn activate(&mut self) {
        self.window.set_cursor_grab(winit::window::CursorGrabMode::Confined).expect("Could not grab cursor");
        self.window.set_cursor_visible(false);
    }

    fn deactivate(&mut self) {
        self.window.set_cursor_grab(winit::window::CursorGrabMode::None).expect("Could not ungrab cursor");
        self.window.set_cursor_visible(true);
    }

    fn window_resized(&mut self, PhysicalSize { width, height }: PhysicalSize<u32>) {
        self.screen_width = width;
        self.screen_height = height;
    }

    fn cursor_moved(&mut self, position: &PhysicalPosition<f64>) {
        let screen_center = winit::dpi::PhysicalPosition::new(
            self.screen_width as f64 / 2.0,
            self.screen_height as f64 / 2.0,
        );
        let pos = cgmath::Vector2::new(position.x as f32, position.y as f32)
            - cgmath::Vector2::new(screen_center.x as f32, screen_center.y as f32);
        let pos = pos / (self.screen_height as f32) * self.mouse_sensitivity;

        self.mouse_x += pos.x;
        self.mouse_x %= 360.0;
        self.mouse_y += pos.y;
        self.mouse_y = self.mouse_y.clamp(-89.0, 89.0);

        self.window.set_cursor_position(screen_center).expect("Could not set cursor position");
    }

    fn handle_event(&mut self, event: &winit::event::KeyEvent) {
        self.inputs_received.push(event.clone());
    }

    fn update(&mut self, dt: f32) {
        // Note: we could tag the input events with a timestamp and only process those within dt range
        for event in self.inputs_received.drain(..) {
            for input in self.inputs.values_mut() {
                let pressed = event.state == winit::event::ElementState::Pressed;
                let physical_key = event.physical_key;
                let key_code = match physical_key {
                    winit::keyboard::PhysicalKey::Code(key_code) => key_code,
                    _ => continue,
                };
                if input.keys.contains(&key_code) {
                    input.key_state.just_pressed = pressed && !input.key_state.pressed;
                    input.key_state.just_released = !pressed && input.key_state.pressed;
                    input.key_state.pressed = pressed;
                }
            }
        }
    }
    
}
