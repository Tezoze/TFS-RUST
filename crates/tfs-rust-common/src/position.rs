use crate::enums::Direction;
use std::cmp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Position {
    pub x: u16,
    pub y: u16,
    pub z: u8,
}

impl Position {
    pub fn new(x: u16, y: u16, z: u8) -> Self {
        Self { x, y, z }
    }

    pub fn offset(&self, dir: Direction) -> Self {
        let mut n = *self;
        match dir {
            Direction::North => n.y = n.y.saturating_sub(1),
            Direction::East => n.x = n.x.saturating_add(1),
            Direction::South => n.y = n.y.saturating_add(1),
            Direction::West => n.x = n.x.saturating_sub(1),
            Direction::SouthWest => {
                n.x = n.x.saturating_sub(1);
                n.y = n.y.saturating_add(1);
            }
            Direction::SouthEast => {
                n.x = n.x.saturating_add(1);
                n.y = n.y.saturating_add(1);
            }
            Direction::NorthWest => {
                n.x = n.x.saturating_sub(1);
                n.y = n.y.saturating_sub(1);
            }
            Direction::NorthEast => {
                n.x = n.x.saturating_add(1);
                n.y = n.y.saturating_sub(1);
            }
        }
        n
    }

    pub fn distance_to(&self, other: &Position) -> u32 {
        if self.z != other.z {
            return u32::MAX; // TFS usually considers different floors unreachable directly
        }
        let dx = (self.x as i32 - other.x as i32).unsigned_abs();
        let dy = (self.y as i32 - other.y as i32).unsigned_abs();
        cmp::max(dx, dy)
    }

    pub fn get_direction_to(&self, other: &Position) -> Direction {
        let dx = other.x as i32 - self.x as i32;
        let dy = other.y as i32 - self.y as i32;

        let mut angle = (dy as f32).atan2(dx as f32) * 180.0 / std::f32::consts::PI;
        if angle < 0.0 {
            angle += 360.0;
        }

        if angle >= 337.5 || (0.0..22.5).contains(&angle) {
            Direction::East
        } else if (22.5..67.5).contains(&angle) {
            Direction::SouthEast
        } else if (67.5..112.5).contains(&angle) {
            Direction::South
        } else if (112.5..157.5).contains(&angle) {
            Direction::SouthWest
        } else if (157.5..202.5).contains(&angle) {
            Direction::West
        } else if (202.5..247.5).contains(&angle) {
            Direction::NorthWest
        } else if (247.5..292.5).contains(&angle) {
            Direction::North
        } else {
            Direction::NorthEast
        }
    }
}
