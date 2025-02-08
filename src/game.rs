

pub mod camera;

use crate::game::camera::Camera;

struct Obstacle {
    start: f32,
    end: f32,
}

struct Rail {
    obstacles: Vec<Obstacle>,
}

#[derive(PartialEq, Eq)]
enum GameState {
    Playing,
    GameOver,
    NewGame,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct PlayerInput {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub confirm: bool,
    pub cancel: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct PlayerInputInternal {
    pub left: bool,
    left_just_pressed: bool,
    pub right: bool,
    right_just_pressed: bool,
    pub up: bool,
    up_just_pressed: bool,
    pub down: bool,
    down_just_pressed: bool,
    pub confirm: bool,
    confirm_just_pressed: bool,
    pub cancel: bool,
    cancel_just_pressed: bool,
}

pub struct Game {
    camera: camera::Camera,
    player_pos: f32, // in radians
    player_width: f32, // in radians
    player_input: PlayerInputInternal,
    player_speed: f32,
    rails: [Rail; 6],
    time: f32,
    state: GameState,
}

impl Game {
    pub fn new(aspect: f32) -> Self {
        let camera = Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 1.0, 2.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: aspect,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
    
        Self {
            camera: camera,
            player_pos: 0.0,
            player_width: 0.3,
            player_input: PlayerInputInternal::default(),
            rails: [
                Rail { obstacles: vec![], },
                Rail { obstacles: vec![], },
                Rail {
                    obstacles: vec![
                        Obstacle { start: 1.0, end: 2.0 },
                        Obstacle { start: 4.0, end: 8.0 },
                    ],
                },
                Rail { obstacles: vec![], },
                Rail { obstacles: vec![], },
                Rail { obstacles: vec![], },
            ],
            time: 0.0,
            player_speed: 1.0,
            state: GameState::NewGame,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;

        if self.state != GameState::Playing {
            return;
        }
        if self.player_input.left {
            self.player_pos -= 0.1 * dt;
        }
        if self.player_input.right {
            self.player_pos += 0.1 * dt;
        }
        // check for collisions
        let player_start = self.player_pos - self.player_width / 2.0;
        let player_end = self.player_pos + self.player_width / 2.0;
        let collided = self.obstacle_at_position(player_start) || self.obstacle_at_position(player_end);
        if collided {
            self.state = GameState::GameOver;
        }
    }

    pub fn set_player_inputs(&mut self, _player_input: &PlayerInput) {
        macro_rules! set_input { //TODO: Remove the field_just_pressed entry, possibly use a loop below.
            ($field:ident, $field_just_pressed:ident) => {
                self.player_input.$field = $field;
                self.player_input.$field_just_pressed = $field && !self.player_input.$field;
            };
        }
        // destructure _player_input into local variables
        let PlayerInput { left, right, up, down, confirm, cancel } = *_player_input;

        set_input!(left, left_just_pressed);
        set_input!(right, right_just_pressed);
        set_input!(up, up_just_pressed);
        set_input!(down, down_just_pressed);
        set_input!(confirm, confirm_just_pressed);
        set_input!(cancel, cancel_just_pressed);
    }


    fn lane_at_position(&self, pos: f32) -> usize {
        let lane = (pos / (2.0 * std::f32::consts::PI) * 6.0).floor() as usize;
        lane % 6
    }
    fn obstacle_at_position(&self, pos: f32) -> bool {
        let lane = self.lane_at_position(pos);
        for obstacle in &self.rails[lane].obstacles {
            if pos >= obstacle.start && pos <= obstacle.end {
                return true;
            }
        }
        false
    }

}
