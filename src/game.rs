



use crate::common::camera::Camera;
use crate::input::{InputHandler, InputGetInterface};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obstacle {
    pub start: f32,
    pub end: f32,
    pub lane: usize,
}

#[derive(Debug, Clone)]
struct Lane {
    obstacles: Vec<Obstacle>,
}

#[derive(PartialEq, Eq)]
enum GameState {
    Playing,
    GameOver,
    NewGame,
}


pub struct Game {
    pub camera: Camera,
    pub player_angle: f32, // in radians
    pub player_width: f32, // in radians
    player_speed: f32,
    lanes: [Lane; 6],
    pub time: f32,
    state: GameState,
    spawner: Box<dyn SpawnerInterface>,
}

trait SpawnerInterface {
    fn update(&mut self, dt: f32, lanes: &mut [Lane]);
}

struct BasicSpawner {
    patterns: Vec<Pattern>,
    current_pattern: usize,
    current_pattern_time: f32,
}
impl BasicSpawner {
    pub const SPAWN_DISTANCE: f32 = 15.0;
}

impl SpawnerInterface for BasicSpawner {
    fn update(&mut self, dt: f32, lanes: &mut [Lane]) {
        self.current_pattern_time += dt;
        let current_pattern = &self.patterns[self.current_pattern];
        if self.current_pattern_time > current_pattern.duration {
            println!("Switching pattern");
            self.current_pattern_time = 0.0;
            self.current_pattern = (self.current_pattern + 1) % self.patterns.len();
            for obstacle in &current_pattern.obstacles {
                lanes[obstacle.lane].obstacles.push(Obstacle {
                    start: Self::SPAWN_DISTANCE + obstacle.start,
                    end: Self::SPAWN_DISTANCE + obstacle.end,
                    lane: obstacle.lane,
                });
            }
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
struct Pattern {
    obstacles: Vec<Obstacle>,
    duration: f32,
}

impl Game {
    pub const DISPLAY_HEIGHT: f32 = 1.0;
    pub const PLAYER_RADIUS: f32 = 3.0;
    pub const OBSTACLE_SPEED: f32 = 4.0;

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

        let mut obstacles1 = vec![];
        for i in 0..12 {
            obstacles1.push( Obstacle { start: (i * 2) as f32, end: (i * 2) as f32 + 4.0, lane: i % 6 } );
        }
        let mut obstacles2 = vec![];
        for i in 0..6 {
            obstacles2.push( Obstacle { start: (i * 2) as f32, end: (i * 2) as f32 + 2.0, lane: i % 6 } );
        }
        for i in 6..12 {
            obstacles2.push( Obstacle { start: (i * 2) as f32, end: (i * 2) as f32 + 2.0, lane: (12-i) % 6 } );
        }
        
        let patterns = vec![
            Pattern {duration: 8.0, obstacles: obstacles1},
            Pattern {duration: 8.0, obstacles: obstacles2},
        ];
        let spawner = BasicSpawner {
            patterns: patterns.clone(),
            current_pattern: 0,
            current_pattern_time: 1e20,
        };
    
        Self {
            camera: camera,
            player_angle: 0.0,
            player_width: 0.3,
            lanes: [
                Lane { obstacles: vec![], },
                Lane { obstacles: vec![], },
                Lane { obstacles: vec![], },
                Lane { obstacles: vec![], },
                Lane { obstacles: vec![], },
                Lane { obstacles: vec![], },
            ],
            time: 0.0,
            player_speed: 4.0,
            state: GameState::Playing,
            spawner: Box::new(spawner) as Box<dyn SpawnerInterface>,
        }
    }

    fn update_camera(&mut self, dt: f32, input: &InputHandler) {
        let angle = input.get_mouse_x().to_radians();
        let angle_y = input.get_mouse_y().to_radians() + 1.3;
        // let angle = 0.0 as f32;
        // let angle_y = self.time * 0.01;
        let direction = cgmath::Vector3::new(
            angle.sin() * angle_y.cos(),
            -angle_y.sin(),
            -angle.cos() * angle_y.cos(),
        );
        // let direction = cgmath::Vector3::new(0.4, -0.6, 0.8).normalize();
        let up = cgmath::Vector3::new(0.0, 1.0, 0.0);
        // Place the camera 20 units behind the direction
        self.camera.target = cgmath::Point3::new(0.0, 0.0, 0.0);
        self.camera.eye = self.camera.target - 20.0 * direction;
        self.camera.up = up;
        self.camera.target = self.camera.eye + direction;
    }


    pub fn player_position(&self) -> cgmath::Vector3<f32> {
        let radius = Self::PLAYER_RADIUS;
        let x = self.player_angle.cos() * radius;
        let y = Self::DISPLAY_HEIGHT;
        let z = self.player_angle.sin() * radius;
        cgmath::Vector3::new(x, y, z)
    }

    pub fn update(&mut self, dt: f32, input: &InputHandler) {
        self.time += dt;

        self.update_camera(dt, input);

        if self.state != GameState::Playing {
            return;
        }

        self.update_obstacles(dt);
        self.update_player(dt, input);
        self.spawner.update(dt, &mut self.lanes);
        // Debug output
        let mut out = String::new();
        let player_lane = self.lane_at_position(self.player_angle);
        for (i, lane) in self.lanes.iter().enumerate() {
            if i == player_lane {
                out += "P";
            } else {
                out += " ";
            }
            let mut obstacle_found = false;
            for obstacle in &lane.obstacles {
                if obstacle.start < Self::PLAYER_RADIUS && obstacle.end > Self::PLAYER_RADIUS {
                    obstacle_found = true;
                }
            }
            if obstacle_found {
                out += "X";
            } else {
                out += "-";
            }
        }
        println!("{out}");
    }

    fn update_obstacles(&mut self, dt: f32) {
        for lane in &mut self.lanes {
            let mut to_remove = 0;
            for obstacle in &mut lane.obstacles {
                obstacle.start -= dt * Self::OBSTACLE_SPEED;
                obstacle.end -= dt * Self::OBSTACLE_SPEED;
                if obstacle.end < 0.0 {
                    to_remove += 1;
                }
            }
            for _ in 0..to_remove {
                lane.obstacles.remove(0);
            }
        }
    }

    fn update_player(&mut self, dt: f32, input: &InputHandler) {
        let left_pressed = input.get_key_state(crate::input::InputID::Left).pressed;
        let right_pressed = input.get_key_state(crate::input::InputID::Right).pressed;
        if left_pressed && !right_pressed {
            let movement = -self.player_speed * dt;
            self.player_angle += movement;
            // let movement = -self.player_speed * dt;
            // // check for collisions
            // let player_start = self.player_angle - self.player_width / 2.0;
            // let collided = self.obstacle_at_position(player_start + movement);

            // if !collided {
            //     self.player_angle += movement;
            // } else {
            //     // move player_start to right side of the lane
            //     let obstacle_lane = self.lane_at_position(player_start + movement);
            //     let obstacle_right_side = (obstacle_lane as f32 + 0.5) * 2.0 * std::f32::consts::PI;
            //     self.player_angle = obstacle_right_side + self.player_width / 2.0;
            // }
            
        }

        if right_pressed && !left_pressed {
            let movement = self.player_speed * dt;
            self.player_angle += movement;
            // let movement = self.player_speed * dt;
            // // check for collisions
            // let player_end = self.player_angle + self.player_width / 2.0;
            // let collided = self.obstacle_at_position(player_end + movement);

            // if !collided {
            //     self.player_angle += movement;
            // } else {
            //     // move player_end to left side of the lane
            //     let obstacle_lane = self.lane_at_position(player_end + movement);
            //     let obstacle_left_side = (obstacle_lane as f32 - 0.5) * 2.0 * std::f32::consts::PI;
            //     self.player_angle = obstacle_left_side - self.player_width / 2.0;
            // }
        }
        self.player_angle = self.player_angle % (2.0 * std::f32::consts::PI);

        // check for collisions
        let player_start = self.player_angle - self.player_width / 2.0;
        let player_end = self.player_angle + self.player_width / 2.0;

        println!("player_start: {}, player_end: {}", player_start, player_end);
        let collided = self.obstacle_at_position(player_start) || self.obstacle_at_position(player_end);
        if collided {
            self.state = GameState::GameOver;
        }
    }

    fn lane_at_position(&self, pos: f32) -> usize {
        let lane = (pos / (2.0 * std::f32::consts::PI) * 6.0 - 1.0).floor() as i32 % 6;
        if lane < 0 {
            return (lane + 6) as usize
        }
        lane as usize
    }

    fn obstacle_at_position(&self, pos: f32) -> bool {
        let lane = self.lane_at_position(pos);
        for obstacle in &self.lanes[lane].obstacles {
            if obstacle.start < Self::PLAYER_RADIUS && obstacle.end > Self::PLAYER_RADIUS {
                return true;
            }
        }
        false
    }

    pub fn get_obstacles_all(&self) -> Vec<Obstacle> {
        self.lanes.iter().flat_map(|lane| lane.obstacles.clone()).collect()
    }

}
