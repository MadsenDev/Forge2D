//! Camera follow system for tracking entities with dead-zone support.

use crate::math::{Camera2D, Vec2};

/// Camera follow behavior configuration.
#[derive(Clone, Copy, Debug)]
pub struct CameraFollow {
    /// Entity to follow (if using entity-based following)
    pub target_entity: Option<crate::world::EntityId>,
    /// Target position to follow (if not using entity)
    pub target_position: Option<Vec2>,
    /// Dead zone size - camera won't move if target is within this area
    pub dead_zone: Vec2,
    /// Maximum camera speed (for smooth following)
    pub max_speed: f32,
    /// Whether to use smooth following (lerp) or instant
    pub smooth: bool,
    /// Smoothing factor (0.0 = instant, 1.0 = very slow)
    pub smooth_factor: f32,
}

impl CameraFollow {
    /// Create a new camera follow configuration.
    pub fn new() -> Self {
        Self {
            target_entity: None,
            target_position: None,
            dead_zone: Vec2::new(100.0, 100.0), // Default dead zone
            max_speed: f32::INFINITY, // No speed limit by default
            smooth: false,
            smooth_factor: 0.1,
        }
    }

    /// Set the entity to follow.
    pub fn follow_entity(mut self, entity: crate::world::EntityId) -> Self {
        self.target_entity = Some(entity);
        self.target_position = None;
        self
    }

    /// Set the position to follow.
    pub fn follow_position(mut self, position: Vec2) -> Self {
        self.target_position = Some(position);
        self.target_entity = None;
        self
    }

    /// Set the dead zone size (camera won't move if target is within this area).
    pub fn with_dead_zone(mut self, width: f32, height: f32) -> Self {
        self.dead_zone = Vec2::new(width, height);
        self
    }

    /// Enable smooth following with the given factor (0.0 = instant, 1.0 = very slow).
    pub fn with_smoothing(mut self, factor: f32) -> Self {
        self.smooth = true;
        self.smooth_factor = factor.clamp(0.0, 1.0);
        self
    }

    /// Set maximum camera speed for smooth following.
    pub fn with_max_speed(mut self, speed: f32) -> Self {
        self.max_speed = speed;
        self
    }
}

impl Default for CameraFollow {
    fn default() -> Self {
        Self::new()
    }
}

/// Update camera to follow target with dead-zone support.
pub fn update_camera_follow(
    camera: &mut Camera2D,
    follow: &CameraFollow,
    physics: &crate::physics::PhysicsWorld,
    dt: f32,
) {
    // Get target position
    let target_pos = if let Some(entity) = follow.target_entity {
        physics.body_position(entity).unwrap_or(camera.position)
    } else if let Some(pos) = follow.target_position {
        pos
    } else {
        return; // No target to follow
    };

    // Calculate offset from camera center to target
    let offset = target_pos - camera.position;
    
    // Check if target is outside dead zone
    let half_dead_zone = Vec2::new(follow.dead_zone.x / 2.0, follow.dead_zone.y / 2.0);
    let outside_dead_zone = offset.x.abs() > half_dead_zone.x || offset.y.abs() > half_dead_zone.y;
    
    if !outside_dead_zone {
        return; // Target is within dead zone, don't move camera
    }

    // Calculate desired camera position (target position clamped to dead zone edges)
    let mut desired_pos = camera.position;
    
    if offset.x.abs() > half_dead_zone.x {
        desired_pos.x = target_pos.x - offset.x.signum() * half_dead_zone.x;
    }
    
    if offset.y.abs() > half_dead_zone.y {
        desired_pos.y = target_pos.y - offset.y.signum() * half_dead_zone.y;
    }

    // Update camera position
    if follow.smooth {
        // Smooth following with lerp
        let diff = desired_pos - camera.position;
        let move_distance = diff.length();
        
        if move_distance > 0.0 {
            let lerp_amount = follow.smooth_factor;
            let new_pos = camera.position + diff * lerp_amount;
            
            // Apply max speed limit if set
            if follow.max_speed.is_finite() {
                let max_move = follow.max_speed * dt;
                if move_distance > max_move {
                    let direction = diff.normalized();
                    camera.position = camera.position + direction * max_move;
                } else {
                    camera.position = new_pos;
                }
            } else {
                camera.position = new_pos;
            }
        }
    } else {
        // Instant following
        camera.position = desired_pos;
    }
    
    // Update camera (handles smooth zoom, shake decay, bounds clamping)
    camera.update(dt);
}

