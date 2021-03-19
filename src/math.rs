

#[derive(Debug, Clone, Copy)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, other: f64) -> Self {
        Self { x: self.x * other, y: self.y * other, z: self.z * other }
    }
}

impl std::ops::Mul<Vec3> for f64 {
    type Output = Vec3;
    fn mul(self, other: Vec3) -> Vec3 {
        Vec3 { x: self * other.x, y: self * other.y, z: self * other.z }
    }
}

impl std::ops::Div<f64> for Vec3 {
    type Output = Self;
    fn div(self, other: f64) -> Self {
        Self { x: self.x / other, y: self.y / other, z: self.z / other }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, other: Self) -> Self {
        Vec3 { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
    }
}

impl<'a, 'b> std::ops::Sub<&'b Vec3> for &'a Vec3 {
    type Output = Vec3;
    fn sub(self, other: &'b Vec3) -> Vec3 {
        Vec3 { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }
}

impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, other: Self) {
        *self = Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }
}

impl std::convert::From<(f64, f64, f64)> for Vec3 {
    fn from(tuple: (f64, f64, f64)) -> Vec3 {
        Vec3 { x: tuple.0, y: tuple.1, z: tuple.2 }
    }
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 {
            x: x,
            y: y,
            z: z
        }
    }
    
    pub const fn zero() -> Vec3 {
        Vec3 { x: 0.0, y: 0.0, z: 0.0 }
    }

    pub fn sum(&self) -> f64 {
        self.x + self.y + self.z
    }

    pub fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Self) -> Vec3 {
        Vec3 { 
            x: (self.y * other.z) - (self.z * other.y),
            y: (self.z * other.x) - (self.x * other.z),
            z: (self.x * other.y) - (self.y * other.x)
        }
    }

    pub fn length_to(&self, other: &Self) -> f64 {
        // ((x2 - x1)2 + (y2 - y1)2 + (z2 - z1)2)1/2
        ( (self.x - other.x) * (self.x - other.x)
        + (self.y - other.y) * (self.y - other.y)
        + (self.z - other.z) * (self.z - other.z)
        ).powf(0.5)
    }

    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let m = self.magnitude();
        Vec3 { x: self.x / m, y: self.y / m, z: self.z / m }
    }

    pub fn normal_vector_toward(&self, other: &Vec3) -> Vec3 {
        (other - self).normalize()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Quat {
    x: f32,
    y: f32,
    z: f32,
    w: f32
}

impl Quat {
    pub const fn zero() -> Quat {
        Quat { x: 0.0, y: 0.0, z: 0.0, w: 0.0 }
    }
}

