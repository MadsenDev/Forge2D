use glam::{Mat4, Vec2 as GlamVec2, Vec3};

/// 2D vector type used throughout Forge2D.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len == 0.0 {
            Self::ZERO
        } else {
            Self::new(self.x / len, self.y / len)
        }
    }

    pub fn to_glam(&self) -> GlamVec2 {
        GlamVec2::new(self.x, self.y)
    }

    /// Returns the squared length of the vector (faster than `length()`).
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Computes the dot product of two vectors.
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    /// Computes the distance between two points.
    pub fn distance(self, rhs: Self) -> f32 {
        (self - rhs).length()
    }

    /// Computes the squared distance between two points (faster than `distance()`).
    pub fn distance_squared(self, rhs: Self) -> f32 {
        (self - rhs).length_squared()
    }

    /// Linearly interpolates between two vectors.
    pub fn lerp(self, rhs: Self, t: f32) -> Self {
        Self::new(
            self.x + (rhs.x - self.x) * t,
            self.y + (rhs.y - self.y) * t,
        )
    }

    /// Creates a unit vector pointing in the given direction (angle in radians).
    pub fn from_angle(angle: f32) -> Self {
        Self::new(angle.cos(), angle.sin())
    }

    /// Returns a vector with component-wise absolute values.
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs())
    }

    /// Returns a vector with component-wise minimum values.
    pub fn min(self, rhs: Self) -> Self {
        Self::new(self.x.min(rhs.x), self.y.min(rhs.y))
    }

    /// Returns a vector with component-wise maximum values.
    pub fn max(self, rhs: Self) -> Self {
        Self::new(self.x.max(rhs.x), self.y.max(rhs.y))
    }
}

impl From<(f32, f32)> for Vec2 {
    fn from(value: (f32, f32)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl std::ops::MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl std::ops::Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl std::ops::DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl std::ops::Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

/// Transform describing 2D position, scale, and rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    pub position: Vec2,
    pub scale: Vec2,
    /// Rotation in radians around the Z axis.
    pub rotation: f32,
}

impl Transform2D {
    pub fn new(position: Vec2, scale: Vec2, rotation: f32) -> Self {
        Self {
            position,
            scale,
            rotation,
        }
    }

    pub fn identity() -> Self {
        Self {
            position: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
        }
    }

    pub fn to_matrix(&self, base_size: Vec2) -> Mat4 {
        let translation = Mat4::from_translation(Vec3::new(self.position.x, self.position.y, 0.0));
        let rotation = Mat4::from_rotation_z(self.rotation);
        let scale = Mat4::from_scale(Vec3::new(
            self.scale.x * base_size.x,
            self.scale.y * base_size.y,
            1.0,
        ));

        translation * rotation * scale
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::identity()
    }
}

/// Camera representing a simple 2D view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
}

impl Camera2D {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            zoom: 1.0,
        }
    }

    pub fn view_projection(&self, width: u32, height: u32) -> Mat4 {
        let projection = Mat4::orthographic_rh_gl(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);

        let translation =
            Mat4::from_translation(Vec3::new(-self.position.x, -self.position.y, 0.0));
        let zoom = Mat4::from_scale(Vec3::new(self.zoom, self.zoom, 1.0));

        projection * zoom * translation
    }

    /// Converts screen coordinates to world coordinates using this camera.
    pub fn screen_to_world(&self, screen_pos: Vec2, _screen_width: u32, _screen_height: u32) -> Vec2 {
        let world_x = (screen_pos.x / self.zoom) + self.position.x;
        let world_y = (screen_pos.y / self.zoom) + self.position.y;
        Vec2::new(world_x, world_y)
    }

    /// Converts world coordinates to screen coordinates using this camera.
    pub fn world_to_screen(&self, world_pos: Vec2, _screen_width: u32, _screen_height: u32) -> Vec2 {
        let screen_x = (world_pos.x - self.position.x) * self.zoom;
        let screen_y = (world_pos.y - self.position.y) * self.zoom;
        Vec2::new(screen_x, screen_y)
    }
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}
