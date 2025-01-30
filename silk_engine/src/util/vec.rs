use std::ops::{self, MulAssign};

use crate::util::Rand;

#[allow(unused)]
pub trait Funcs: Sized + Rand + MulAssign + From<f32> + Copy {
    fn step(self, b: Self) -> Self;
    fn noise(self) -> f32;
    fn fbm(self, oct: u32) -> f32 {
        let mut s = 0.0;
        let mut m = 0.0;
        let mut a = 0.5;
        let mut p = self;
        for _ in 0..oct {
            s += a * p.noise();
            m += a;
            a *= 0.5;
            p *= Self::from(2.0);
        }
        return s / m;
    }
    fn smooth(self) -> Self;
    fn lerp(self, other: Self, k: Self) -> Self;
    fn slerp(self, b: Self, k: Self) -> Self;
    fn sstep(self, b: Self, k: Self) -> Self;
    fn bezier(self, b: Self, c: Self, t: f32) -> Self;
}

impl Funcs for f32 {
    fn step(self, b: Self) -> Self {
        (b > self) as i32 as f32
    }

    fn noise(self) -> f32 {
        let fl = self.abs().floor();
        let fr = self.abs().fract();
        fl.rand().slerp((fl + 1.0).rand(), fr.smooth())
    }

    fn smooth(self) -> Self {
        self * self * (3.0 - 2.0 * self)
    }

    fn lerp(self, other: Self, k: Self) -> Self {
        self + (other - self) * k
    }

    fn slerp(self, b: Self, k: Self) -> Self {
        self.lerp(b, k.smooth())
    }

    fn sstep(self, e0: Self, e1: Self) -> Self {
        ((self - e0) / (e1 - e0)).clamp(0.0, 1.0).smooth()
    }

    fn bezier(self, b: Self, c: Self, t: f32) -> Self {
        let a = self;
        t * (t * (c - 2.0 * b + a) + 2.0 * (b - a)) + a
    }
}

#[derive(Clone)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl ops::Add<&Vec2> for Vec2 {
    type Output = Self;
    fn add(self, rhs: &Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl ops::Add<Vec2> for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl ops::Add<f32> for Vec2 {
    type Output = Self;
    fn add(self, rhs: f32) -> Self {
        Self {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}

impl ops::AddAssign<&Vec2> for Vec2 {
    fn add_assign(&mut self, rhs: &Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl ops::AddAssign<Vec2> for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl ops::AddAssign<f32> for Vec2 {
    fn add_assign(&mut self, rhs: f32) {
        self.x += rhs;
        self.y += rhs;
    }
}

impl ops::Sub<&Vec2> for Vec2 {
    type Output = Self;
    fn sub(self, rhs: &Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl ops::Sub<Vec2> for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl ops::Sub<f32> for Vec2 {
    type Output = Self;
    fn sub(self, rhs: f32) -> Self {
        Self {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl ops::SubAssign<&Vec2> for Vec2 {
    fn sub_assign(&mut self, rhs: &Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl ops::SubAssign<Vec2> for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl ops::SubAssign<f32> for Vec2 {
    fn sub_assign(&mut self, rhs: f32) {
        self.x -= rhs;
        self.y -= rhs;
    }
}

impl ops::Mul<&Vec2> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: &Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl ops::Mul<Vec2> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl ops::MulAssign<&Vec2> for Vec2 {
    fn mul_assign(&mut self, rhs: &Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl ops::MulAssign<Vec2> for Vec2 {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl ops::MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl ops::Div<&Vec2> for Vec2 {
    type Output = Self;
    fn div(self, rhs: &Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

impl ops::Div<Vec2> for Vec2 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

impl ops::Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl ops::DivAssign<&Vec2> for Vec2 {
    fn div_assign(&mut self, rhs: &Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl ops::DivAssign<Vec2> for Vec2 {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl ops::DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl ops::Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };
    pub const NEG_ONE: Self = Self { x: -1.0, y: -1.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0 };
    pub const NEG_X: Self = Self { x: -1.0, y: 0.0 };
    pub const NEG_Y: Self = Self { x: 0.0, y: -1.0 };

    pub fn splat(v: f32) -> Self {
        Self::new(v, v)
    }

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn angle(a: f32) -> Self {
        Self::new(a.cos(), a.sin())
    }

    pub fn zero() -> Self {
        Self::splat(0.0)
    }

    pub fn one() -> Self {
        Self::splat(1.0)
    }

    pub fn dot(&self, rhs: &Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    pub fn len2(&self) -> f32 {
        self.dot(self)
    }

    pub fn len(&self) -> f32 {
        self.len2().sqrt()
    }

    pub fn norm(&self) -> Self {
        self.clone() / self.len()
    }

    pub fn dist2(&self, rhs: &Self) -> f32 {
        (rhs.clone() - self).len2()
    }

    pub fn dist(&self, rhs: &Self) -> f32 {
        (rhs.clone() - self).len()
    }

    pub fn sign(&self) -> Self {
        Self::new(self.x.signum(), self.y.signum())
    }

    pub fn abs(&self) -> Self {
        Self::new(self.x.abs(), self.y.abs())
    }

    pub fn floor(&self) -> Self {
        Self::new(self.x.floor(), self.y.floor())
    }

    pub fn round(&self) -> Self {
        Self::new(self.x.round(), self.y.round())
    }

    pub fn ceil(&self) -> Self {
        Self::new(self.x.ceil(), self.y.ceil())
    }

    pub fn fract(&self) -> Self {
        Self::new(self.x.fract(), self.y.fract())
    }

    pub fn exp(&self) -> Self {
        Self::new(self.x.exp(), self.y.exp())
    }

    pub fn pow(&self, p: f32) -> Self {
        Self::new(self.x.powf(p), self.y.powf(p))
    }

    pub fn lerp(&self, rhs: &Self, k: f32) -> Self {
        Self::new(self.x.lerp(rhs.x, k), self.y.lerp(rhs.y, k))
    }

    pub fn slerp(&self, rhs: &Self, k: &Self) -> Self {
        Self::new(self.x.slerp(rhs.x, k.x), self.y.slerp(rhs.y, k.y))
    }

    pub fn sstep(self, e0: Self, e1: Self) -> Self {
        Self::new(self.x.sstep(e0.x, e1.x), self.y.sstep(e0.y, e1.y))
    }

    pub fn step(&self, rhs: &Self) -> Self {
        Self::new(self.x.step(rhs.x), self.y.step(rhs.y))
    }

    pub fn angle_between(&self, rhs: &Self) -> f32 {
        self.norm().dot(&rhs.norm())
    }

    pub fn max(&self, rhs: &Self) -> Self {
        Self::new(self.x.max(rhs.x), self.y.max(rhs.y))
    }

    pub fn max_elem(&self) -> f32 {
        self.x.max(self.y)
    }

    pub fn min(&self, rhs: &Self) -> Self {
        Self::new(self.x.min(rhs.x), self.y.min(rhs.y))
    }

    pub fn min_elem(&self) -> f32 {
        self.x.min(self.y)
    }

    pub fn sin(&self) -> Self {
        Self::new(self.x.sin(), self.y.sin())
    }

    pub fn cos(&self) -> Self {
        Self::new(self.x.cos(), self.y.cos())
    }

    pub fn clamp(&self, min: &Self, max: &Self) -> Self {
        Self::new(self.x.max(min.x).min(max.x), self.y.max(min.y).min(max.y))
    }

    pub fn smooth(&self) -> Self {
        Self::new(self.x.smooth(), self.y.smooth())
    }

    pub fn rand(&self) -> f32 {
        ((self.x * 12.9898 + self.y * 4.1414).sin() * 43758.547).fract()
    }

    pub fn noise(&self) -> f32 {
        let ip = self.floor();
        let u = self.fract().smooth();
        let res = ip
            .rand()
            .lerp(Vec2::new(ip.x + 1.0, ip.y).rand(), u.x)
            .lerp(
                Vec2::new(ip.x, ip.y + 1.0)
                    .rand()
                    .lerp((ip + 1.0).rand(), u.x),
                u.y,
            );
        res * res
    }

    pub fn snoise(&self) -> f32 {
        let rand = |p: &Vec2| -> Vec2 {
            (Vec2::new(
                p.dot(&Vec2::new(127.1, 311.7)),
                p.dot(&Vec2::new(269.5, 183.3)),
            )
            .sin()
                * 43758.547)
                .fract()
                * 2.0
                - 1.0
        };

        let i = (self.clone() + (self.x + self.y) * 0.36602542).floor();
        let a = self.clone() - &i + (i.x + i.y) * 0.21132487;
        let m = a.y.step(a.x);
        let o = Vec2::new(m, 1.0 - m);
        let b = a.clone() - &o + 0.21132487;
        let c = a.clone() - 1.0 + 2.0 * 0.21132487;
        let h = (-Vec3::new(a.len2(), b.len2(), c.len2()) + 0.5).max(&Vec3::splat(0.0));
        let n = h.clone()
            * &h
            * &h
            * &h
            * Vec3::new(
                a.dot(&rand(&i)),
                b.dot(&rand(&(i.clone() + o))),
                c.dot(&rand(&(i + 1.0))),
            );
        n.dot(&Vec3::splat(70.0)) * 0.5 + 0.5
    }

    pub fn voronoise(&self, smooth: f32) -> f32 {
        let hash3 = |p: &Self| {
            let q = Vec3::new(
                p.dot(&Vec2::new(127.1, 311.7)),
                p.dot(&Vec2::new(269.5, 183.3)),
                p.dot(&Vec2::new(419.2, 371.9)),
            );
            (q.sin() * 43758.547).fract()
        };
        let p = self.floor();
        let f = self.fract();
        let k = 1.0 + 63.0 * (-smooth + 1.0).powf(4.0);
        let mut va = 0.0;
        let mut wt = 0.0;
        for j in -2..=2 {
            for i in -2..=2 {
                let g = Vec2::new(i as f32, j as f32);
                let o = hash3(&(p.clone() + &g)) * Vec3::splat(1.0);
                let r = g - &f + Vec2::new(o.x, o.y);
                let d = r.len2();
                let ww = (1.0 - d.sqrt().sstep(0.0, std::f32::consts::SQRT_2)).powf(k);
                va += o.z * ww;
                wt += ww;
            }
        }
        va / wt
    }

    pub fn fbm(&self, oct: u32, amp_mul: f32, freq_mul: f32) -> f32 {
        let mut s = 0.0;
        let mut m = 0.0;
        let mut a = 1.0;
        let mut p = self.clone();

        for _ in 0..oct {
            s += a * p.noise();
            m += a;
            a *= amp_mul;
            p *= freq_mul;
        }
        s / m
    }

    pub fn sfbm(self, oct: u32, amp_mul: f32, freq_mul: f32) -> f32 {
        let mut s = 0.0;
        let mut m = 0.0;
        let mut a = 1.0;
        let mut p = self.clone();

        for _ in 0..oct {
            s += a * p.snoise();
            m += a;
            a *= amp_mul;
            p *= freq_mul;
        }
        s / m
    }
}
#[derive(Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl ops::Add<&Vec3> for Vec3 {
    type Output = Self;
    fn add(self, rhs: &Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl ops::Add<Vec3> for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl ops::Add<f32> for Vec3 {
    type Output = Self;
    fn add(self, rhs: f32) -> Self {
        Self {
            x: self.x + rhs,
            y: self.y + rhs,
            z: self.z + rhs,
        }
    }
}

impl ops::AddAssign<&Vec3> for Vec3 {
    fn add_assign(&mut self, rhs: &Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl ops::AddAssign<Vec3> for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl ops::AddAssign<f32> for Vec3 {
    fn add_assign(&mut self, rhs: f32) {
        self.x += rhs;
        self.y += rhs;
        self.z += rhs;
    }
}

impl ops::Sub<&Vec3> for Vec3 {
    type Output = Self;
    fn sub(self, rhs: &Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl ops::Sub<Vec3> for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl ops::Sub<f32> for Vec3 {
    type Output = Self;
    fn sub(self, rhs: f32) -> Self {
        Self {
            x: self.x - rhs,
            y: self.y - rhs,
            z: self.z - rhs,
        }
    }
}

impl ops::SubAssign<&Vec3> for Vec3 {
    fn sub_assign(&mut self, rhs: &Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl ops::SubAssign<Vec3> for Vec3 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl ops::SubAssign<f32> for Vec3 {
    fn sub_assign(&mut self, rhs: f32) {
        self.x -= rhs;
        self.y -= rhs;
        self.z -= rhs;
    }
}

impl ops::Mul<&Vec3> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: &Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl ops::Mul<Vec3> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl ops::MulAssign<&Vec3> for Vec3 {
    fn mul_assign(&mut self, rhs: &Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.z *= rhs.z;
    }
}

impl ops::MulAssign<Vec3> for Vec3 {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.z *= rhs.z;
    }
}

impl ops::MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}

impl ops::Div<&Vec3> for Vec3 {
    type Output = Self;
    fn div(self, rhs: &Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        }
    }
}

impl ops::Div<Vec3> for Vec3 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        }
    }
}

impl ops::Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl ops::DivAssign<&Vec3> for Vec3 {
    fn div_assign(&mut self, rhs: &Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
    }
}

impl ops::DivAssign<Vec3> for Vec3 {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
    }
}

impl ops::DivAssign<f32> for Vec3 {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
        self.z /= rhs;
    }
}

impl ops::Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const ONE: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    pub const NEG_ONE: Self = Self {
        x: -1.0,
        y: -1.0,
        z: -1.0,
    };
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    pub const NEG_X: Self = Self {
        x: -1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const NEG_Y: Self = Self {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    };
    pub const NEG_Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };

    pub fn splat(v: f32) -> Self {
        Self::new(v, v, v)
    }

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn angle(a: f32, b: f32) -> Self {
        let bc = b.cos();
        Self::new(a.cos() * bc, a.sin() * bc, b.sin())
    }

    pub fn dot(&self, rhs: &Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn len2(&self) -> f32 {
        self.dot(self)
    }

    pub fn len(&self) -> f32 {
        self.len2().sqrt()
    }

    pub fn norm(&self) -> Self {
        self.clone() / self.len()
    }

    pub fn dist2(&self, rhs: &Self) -> f32 {
        (rhs.clone() - self).len2()
    }

    pub fn dist(&self, rhs: &Self) -> f32 {
        (rhs.clone() - self).len()
    }

    pub fn sign(&self) -> Self {
        Self::new(self.x.signum(), self.y.signum(), self.z.signum())
    }

    pub fn abs(&self) -> Self {
        Self::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    pub fn floor(&self) -> Self {
        Self::new(self.x.floor(), self.y.floor(), self.z.floor())
    }

    pub fn round(&self) -> Self {
        Self::new(self.x.round(), self.y.round(), self.z.round())
    }

    pub fn ceil(&self) -> Self {
        Self::new(self.x.ceil(), self.y.ceil(), self.z.ceil())
    }

    pub fn fract(&self) -> Self {
        Self::new(self.x.fract(), self.y.fract(), self.z.fract())
    }

    pub fn exp(&self) -> Self {
        Self::new(self.x.exp(), self.y.exp(), self.z.exp())
    }

    pub fn pow(&self, p: f32) -> Self {
        Self::new(self.x.powf(p), self.y.powf(p), self.z.powf(p))
    }

    pub fn lerp(&self, rhs: &Self, k: f32) -> Self {
        Self::new(
            self.x.lerp(rhs.x, k),
            self.y.lerp(rhs.y, k),
            self.z.lerp(rhs.z, k),
        )
    }

    pub fn slerp(&self, rhs: &Self, k: &Self) -> Self {
        Self::new(
            self.x.slerp(rhs.x, k.x),
            self.y.slerp(rhs.y, k.y),
            self.z.slerp(rhs.z, k.z),
        )
    }

    pub fn sstep(self, e0: Self, e1: Self) -> Self {
        Self::new(
            self.x.sstep(e0.x, e1.x),
            self.y.sstep(e0.y, e1.y),
            self.z.sstep(e0.z, e1.z),
        )
    }

    pub fn step(&self, rhs: &Self) -> Self {
        Self::new(self.x.step(rhs.x), self.y.step(rhs.y), self.z.step(rhs.z))
    }

    pub fn angle_between(&self, rhs: &Self) -> f32 {
        self.norm().dot(&rhs.norm())
    }

    pub fn max(&self, rhs: &Self) -> Self {
        Self::new(self.x.max(rhs.x), self.y.max(rhs.y), self.z.max(rhs.z))
    }

    pub fn max_elem(&self) -> f32 {
        self.x.max(self.y).max(self.z)
    }

    pub fn min(&self, rhs: &Self) -> Self {
        Self::new(self.x.min(rhs.x), self.y.min(rhs.y), self.z.min(rhs.z))
    }

    pub fn min_elem(&self) -> f32 {
        self.x.min(self.y).min(self.z)
    }

    pub fn sin(&self) -> Self {
        Self::new(self.x.sin(), self.y.sin(), self.z.sin())
    }

    pub fn cos(&self) -> Self {
        Self::new(self.x.cos(), self.y.cos(), self.z.cos())
    }

    pub fn clamp(&self, min: &Self, max: &Self) -> Self {
        Self::new(
            self.x.max(min.x).min(max.x),
            self.y.max(min.y).min(max.y),
            self.z.max(min.z).min(max.z),
        )
    }

    pub fn smooth(&self) -> Self {
        Self::new(self.x.smooth(), self.y.smooth(), self.z.smooth())
    }
}
