#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn len_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn len(self) -> f32 {
        self.len_sq().sqrt()
    }

    pub fn normalize(self) -> Self {
        let l = self.len();
        if l > 0.0 {
            Self {
                x: self.x / l,
                y: self.y / l,
            }
        } else {
            Self::default()
        }
    }

    /// Compute distance from this point to a line segment defined by end points `a` and `b`.
    pub fn distance_to_segment(&self, a: Vec2, b: Vec2) -> f32 {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len_sq = dx * dx + dy * dy;
        if len_sq == 0.0 {
            let rx = self.x - a.x;
            let ry = self.y - a.y;
            return (rx * rx + ry * ry).sqrt();
        }

        let t = ((self.x - a.x) * dx + (self.y - a.y) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);

        let proj_x = a.x + t * dx;
        let proj_y = a.y + t * dy;

        let rx = self.x - proj_x;
        let ry = self.y - proj_y;
        (rx * rx + ry * ry).sqrt()
    }

    /// Compute perpendicular vector (rotated 90 degrees counter-clockwise).
    pub fn perpendicular(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    /// Project this vector onto another vector `other`.
    pub fn project_onto(self, other: Self) -> Self {
        let dot = self.x * other.x + self.y * other.y;
        let len_sq = other.len_sq();
        if len_sq == 0.0 {
            return Self::default();
        }
        let scale = dot / len_sq;
        Self {
            x: other.x * scale,
            y: other.y * scale,
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl std::ops::Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size2 {
    pub w: f32,
    pub h: f32,
}

impl Size2 {
    pub fn new(w: f32, h: f32) -> Self {
        Self { w, h }
    }

    /// Compute the 4 corners of a rectangle of this size centered at the origin.
    pub fn corners(&self) -> [Vec2; 4] {
        let half_w = self.w / 2.0;
        let half_h = self.h / 2.0;
        [
            Vec2::new(-half_w, -half_h),
            Vec2::new(half_w, -half_h),
            Vec2::new(half_w, half_h),
            Vec2::new(-half_w, half_h),
        ]
    }

    /// Check if a point is contained inside a rectangle of this size centered at origin.
    pub fn contains_point(&self, point: Vec2) -> bool {
        let half_w = self.w / 2.0;
        let half_h = self.h / 2.0;
        point.x >= -half_w && point.x <= half_w && point.y >= -half_h && point.y <= half_h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_to_segment() {
        let p = Vec2::new(5.0, 5.0);
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 0.0);
        assert_eq!(p.distance_to_segment(a, b), 5.0);

        let p2 = Vec2::new(15.0, 0.0);
        assert_eq!(p2.distance_to_segment(a, b), 5.0);
    }

    #[test]
    fn test_perpendicular_and_project() {
        let v = Vec2::new(3.0, 4.0);
        assert_eq!(v.perpendicular(), Vec2::new(-4.0, 3.0));

        let onto = Vec2::new(1.0, 0.0);
        assert_eq!(v.project_onto(onto), Vec2::new(3.0, 0.0));
    }

    #[test]
    fn test_size2_helpers() {
        let s = Size2::new(10.0, 20.0);
        assert!(s.contains_point(Vec2::new(0.0, 0.0)));
        assert!(s.contains_point(Vec2::new(5.0, 10.0)));
        assert!(!s.contains_point(Vec2::new(6.0, 0.0)));
    }
}

