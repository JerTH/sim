// Impl's organized in order:
// -struct declaration
// -impl
// -conversions
// -indexing
// -add, addassign
// -sub, subassign
// -mul, mulassign
// -div, divassign

/// Single precsion 3D vector
#[derive(Debug, Clone, Copy, Default)]
pub struct SVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl SVec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x: x,
            y: y,
            z: z
        }
    }
    
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }

    pub fn sum(&self) -> f32 {
        self.x + self.y + self.z
    }

    pub fn dot(&self, rhs: &Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(&self, rhs: &Self) -> Self {
        Self { 
            x: (self.y * rhs.z) - (self.z * rhs.y),
            y: (self.z * rhs.x) - (self.x * rhs.z),
            z: (self.x * rhs.y) - (self.y * rhs.x)
        }
    }

    pub fn length_to(&self, rhs: &Self) -> f32 {
        // ((x2 - x1)2 + (y2 - y1)2 + (z2 - z1)2)1/2
        ( (self.x - rhs.x) * (self.x - rhs.x)
        + (self.y - rhs.y) * (self.y - rhs.y)
        + (self.z - rhs.z) * (self.z - rhs.z)
        ).powf(0.5)
    }
    
    pub fn magnitude(&self) -> f32 {
        self.dot(&self).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let m = self.magnitude();
        Self { x: self.x / m, y: self.y / m, z: self.z / m }
    }
}

impl std::convert::From<DVec3> for SVec3 {
    fn from(dvec: DVec3) -> Self {
        Self { x: dvec.x as f32, y: dvec.y as f32, z: dvec.z as f32 }
    }
}

impl std::convert::From<&DVec3> for SVec3 {
    fn from(dvec: &DVec3) -> Self {
        Self { x: dvec.x as f32, y: dvec.y as f32, z: dvec.z as f32, }
    }
}

impl std::ops::Index<usize> for &SVec3 {
    type Output = f32;
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("SVec3::index out of bounds")
        }
    }
}

impl std::ops::Add for SVec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::Output { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl std::ops::AddAssign for SVec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl std::ops::Sub for SVec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }
}

impl std::ops::Mul<f32> for SVec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        let rhs = rhs as f32;
        Self::Output { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs }
    }
}

impl std::ops::Mul<f64> for SVec3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        let rhs = rhs as f32;
        Self::Output { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs }
    }
}

impl std::ops::Mul<SVec3> for f64 {
    type Output = SVec3;
    fn mul(self, rhs: SVec3) -> Self::Output {
        let scalar = self as f32;
        Self::Output { x: scalar * rhs.x, y: scalar * rhs.y, z: scalar * rhs.z }
    }
}

impl std::ops::Div<f32> for SVec3 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::Output { x: self.x / rhs, y: self.y / rhs, z: self.z / rhs }
    }
}

impl std::ops::Div<f64> for SVec3 {
    type Output = Self;
    fn div(self, rhs: f64) -> Self::Output {
        let rhs = rhs as f32;
        Self::Output { x: self.x / rhs, y: self.y / rhs, z: self.z / rhs }
    }
}

/// Double precision 3D vector
#[derive(Debug, Clone, Copy, Default)]
pub struct DVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64
}

impl DVec3 {
    pub fn new(x: f64, y: f64, z: f64) -> DVec3 {
        DVec3 {
            x: x,
            y: y,
            z: z
        }
    }
    
    pub const fn zero() -> DVec3 {
        DVec3 { x: 0.0, y: 0.0, z: 0.0 }
    }

    pub fn sum(&self) -> f64 {
        self.x + self.y + self.z
    }

    pub fn dot(&self, rhs: &Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(&self, rhs: &Self) -> DVec3 {
        DVec3 { 
            x: (self.y * rhs.z) - (self.z * rhs.y),
            y: (self.z * rhs.x) - (self.x * rhs.z),
            z: (self.x * rhs.y) - (self.y * rhs.x)
        }
    }

    pub fn length_to(&self, rhs: &Self) -> f64 {
        // ((x2 - x1)2 + (y2 - y1)2 + (z2 - z1)2)1/2
        ( (self.x - rhs.x) * (self.x - rhs.x)
        + (self.y - rhs.y) * (self.y - rhs.y)
        + (self.z - rhs.z) * (self.z - rhs.z)
        ).powf(0.5)
    }

    pub fn magnitude(&self) -> f64 {
        self.dot(&self).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let m = self.magnitude();
        DVec3 { x: self.x / m, y: self.y / m, z: self.z / m }
    }

    pub fn normal_vector_toward(&self, rhs: &DVec3) -> DVec3 {
        (rhs - self).normalize()
    }

    pub fn rotate_by(&self, rotation: &Quat) -> Self {
        let result: Quat = *rotation * Quat { w: 0.0, v: self.into() } * rotation.inverse_unit();
        result.v.into()
    }
}

impl std::convert::From<SVec3> for DVec3 {
    fn from(svec: SVec3) -> Self {
        Self { x: svec.x as f64, y: svec.y as f64, z: svec.z as f64 }
    }
}

impl std::convert::From<&SVec3> for DVec3 {
    fn from(svec: &SVec3) -> Self {
        Self { x: svec.x as f64, y: svec.y as f64, z: svec.z as f64 }
    }
}

impl std::convert::From<(f64, f64, f64)> for DVec3 {
    fn from(tuple: (f64, f64, f64)) -> DVec3 {
        DVec3 { x: tuple.0, y: tuple.1, z: tuple.2 }
    }
}

impl std::ops::Index<usize> for &DVec3 {
    type Output = f64;
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("SVec3::index out of bounds")
        }
    }
}

impl std::ops::Mul<f64> for DVec3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self::Output { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs }
    }
}

impl std::ops::Div<f64> for DVec3 {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self { x: self.x / rhs, y: self.y / rhs, z: self.z / rhs }
    }
}

impl std::ops::Sub for DVec3 {
    type Output = DVec3;
    fn sub(self, rhs: Self) -> Self {
        DVec3 { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }
}

impl<'a, 'b> std::ops::Sub<&'b DVec3> for &'a DVec3 {
    type Output = DVec3;
    fn sub(self, rhs: &'b DVec3) -> DVec3 {
        DVec3 { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }
}

impl std::ops::Add for DVec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl std::ops::Add<SVec3> for DVec3 {
    type Output = Self;
    fn add(self, rhs: SVec3) -> Self {
        Self { x: self.x + rhs.x as f64, y: self.y + rhs.y as f64, z: self.z + rhs.z as f64 }
    }
}

impl std::ops::AddAssign for DVec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AxisAngle {
    axis: SVec3,
    angle: f32,
}

impl AxisAngle {
    pub fn new(axis: SVec3, angle: f32) -> Self {
        AxisAngle { axis: axis, angle: angle }
    }

    pub fn quat(&self) -> Quat {
        Quat::from(*self)
    }
}

impl std::convert::From<Quat> for AxisAngle {
    fn from(quaternion: Quat) -> Self {
        const MIN_S: f32 = 0.00001;
        
        let q = quaternion.renormalize();
        let s = (1.0 - (q.w * q.w)).sqrt();
        let angle = 2.0 * q.w.acos();
        
        if s < MIN_S {
            AxisAngle::new(SVec3::new(q.v.x, q.v.y, q.v.z), angle)
        } else {
            AxisAngle::new(SVec3::new(q.v.x / s, q.v.y / s, q.v.z / s), angle)
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Quat {
    w: f32, // real part

    // i^2 = j^2 = k^2 = ijk = -1
    // ij = k
    // jk = i
    // ki = j
    // ji = -k
    // kj = -i
    // ik = -j
    v: SVec3, // (i, j, k)
}

impl Quat {
    pub const fn zero() -> Quat {
        Quat { w: 0.0, v: SVec3::zero() }
    }

    pub fn axis_angle(&self) -> AxisAngle {
        AxisAngle::from(*self)
    }

    pub fn dot(&self, rhs: &Self) -> f32 {
        self.w * rhs.w + self.v.dot(&rhs.v)
    }

    pub fn magnitude(&self) -> f32 {
        self.dot(&self).sqrt()
    }

    pub fn normalize(&self) -> Quat {
        let magnitude: f32 = self.magnitude();
        Quat { w: self.w / magnitude, v: self.v / magnitude }
    }

    pub fn renormalize(&self) -> Quat {
        let sq_len: f32 = self.w * self.w + self.v.dot(&self.v);
        let inv_sq_len: f32 = Quat::fast_unit_inv_sqrt(sq_len);
        Quat { w: self.w * inv_sq_len, v: self.v * inv_sq_len }
    }

    pub fn inverse_unit(&self) -> Quat {
        Quat { w: self.w, v: self.v * -1.0 }
    }

    fn fast_unit_inv_sqrt(x: f32) -> f32 {
        const A: f32 = 15.0 / 8.0;
        const B: f32 = -5.0 / 4.0;
        const C: f32 = 3.0 / 8.0;
        
        A + (B * x) + (C * x * x)
    }
}

impl std::convert::From<AxisAngle> for Quat {
    fn from(axis_angle: AxisAngle) -> Self {
        let axis = axis_angle.axis;
        let angle = axis_angle.angle;
        let half_angle = angle / 2.0;
        let sin_half_angle = half_angle.sin();

        Quat {
            w: angle.cos(),
            v: SVec3 { x: axis.x * sin_half_angle, y: axis.y * sin_half_angle, z: axis.z * sin_half_angle }
        }.normalize()
    }
}

impl std::ops::Add for Quat {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Quat { w: self.w + rhs.w, v: self.v + rhs.v }
    }
}

impl std::ops::Sub for Quat {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Quat { w: self.w - rhs.w, v: self.v - rhs.v }
    }
}

impl std::ops::Mul for Quat {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let w: f32 = self.w * rhs.w - self.v.dot(&rhs.v);
        let v: SVec3 = (self.w * rhs.v) + (rhs.w * self.v) + self.v.cross(&rhs.v);
        Quat { w: w, v: v }
    }
}

impl std::ops::Mul<f32> for Quat {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Quat { w: self.w * rhs, v: self.v * rhs }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SMatrix3x3 {
    c0: SVec3,
    c1: SVec3,
    c2: SVec3,
}

impl std::ops::Index<usize> for &SMatrix3x3 {
    type Output = SVec3;
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.c0,
            1 => &self.c1,
            2 => &self.c2,
            _ => panic!("SVec3::index out of bounds")
        }
    }
}

// Built-in type extensions & conversions

impl std::ops::Mul<DVec3> for f64 {
    type Output = DVec3;
    fn mul(self, rhs: Self::Output) -> Self::Output {
        Self::Output { x: self * rhs.x, y: self * rhs.y, z: self * rhs.z }
    }
}

impl std::ops::Mul<SVec3> for f32 {
    type Output = SVec3;
    fn mul(self, rhs: Self::Output) -> Self::Output {
        Self::Output { x: self * rhs.x, y: self * rhs.y, z: self * rhs.z }
    }
}
