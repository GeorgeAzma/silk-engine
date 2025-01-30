#![allow(unused)]
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use super::rand::{Noise, Rand};

pub trait ExtraFns:
    Sized + Rand + Copy + Add<Self, Output = Self> + Mul<Self, Output = Self> + Sub<Self, Output = Self>
{
    fn step(self, b: Self) -> Self;
    fn smooth(self) -> Self;
    fn lerp(self, other: Self, k: f32) -> Self;
    fn sstep(self, e0: Self, e1: Self) -> Self;
    fn saturate(self) -> Self;
}

impl ExtraFns for f32 {
    fn step(self, b: Self) -> Self {
        (b > self) as i32 as f32
    }

    fn smooth(self) -> Self {
        self * self * (3.0 - 2.0 * self)
    }

    fn lerp(self, other: Self, k: Self) -> Self {
        self + (other - self) * k
    }

    fn sstep(self, e0: Self, e1: Self) -> Self {
        ((self - e0) / (e1 - e0)).clamp(0.0, 1.0).smooth()
    }

    fn saturate(self) -> Self {
        self.clamp(0.0, 1.0)
    }
}

pub trait Bezier {
    fn bezier(self, b: Self, c: Self, t: f32) -> Self;
}

impl Bezier for f32 {
    fn bezier(self, b: Self, c: Self, t: f32) -> Self {
        let a = self;
        t * (t * (c - 2.0 * b + a) + 2.0 * (b - a)) + a
    }
}

macro_rules! impl_op {
    ($trait: ident, $method: ident, $op: tt, $ty: ident, $comp_ty: ty, $($comp: ident),+) => {
        impl $trait<&$ty> for $ty {
            type Output = Self;
            #[inline(always)]
            fn $method(self, rhs: &Self) -> Self {
                Self {
                    $($comp: self.$comp $op rhs.$comp),*
                }
            }
        }

        impl $trait<$ty> for $ty {
            type Output = Self;
            #[inline(always)]
            fn $method(self, rhs: Self) -> Self {
                Self {
                    $($comp: self.$comp $op rhs.$comp),*
                }
            }
        }

        impl $trait<$comp_ty> for $ty {
            type Output = Self;
            fn $method(self, rhs: $comp_ty) -> Self {
                Self {
                    $($comp: self.$comp $op rhs),*
                }
            }
        }

        impl $trait<$ty> for $comp_ty {
            type Output = $ty;
            #[inline(always)]
            fn $method(self, rhs: $ty) -> Self::Output {
                rhs $op self
            }
        }
    };
}

macro_rules! impl_op_assign {
    ($trait: ident, $method: ident, $op: tt, $ty: ident, $comp_ty: ty, $($comp: ident),+) => {
        impl $trait<&$ty> for $ty {
            fn $method(&mut self, rhs: &Self) {
                $(self.$comp $op rhs.$comp;)*
            }
        }

        impl $trait<$ty> for $ty {
            fn $method(&mut self, rhs: Self) {
                $(self.$comp $op rhs.$comp;)*
            }
        }

        impl $trait<$comp_ty> for $ty {
            fn $method(&mut self, rhs: $comp_ty) {
                $(self.$comp $op rhs;)*
            }
        }
    };
}

macro_rules! impl_vec_fn {
    ($method: ident, $($comp: ident),+) => {
        fn $method(mut self) -> Self {
            $(self.$comp = self.$comp.$method();)*
            self
        }
    }
}

macro_rules! impl_vecu {
    ($($comp: ident),+) => {
        fn max(mut self, rhs: Self) -> Self {
            $(self.$comp = self.$comp.max(rhs.$comp);)*
            self
        }

        fn min(mut self, rhs: Self) -> Self {
            $(self.$comp = self.$comp.min(rhs.$comp);)*
            self
        }

        fn clamp(mut self, min: Self, max: Self) -> Self {
            $(self.$comp = self.$comp.clamp(min.$comp, max.$comp);)*
            self
        }
    };
}

macro_rules! impl_veci {
    ($($comp: ident),+) => {
        impl_vecu!($($comp),*);

        impl_vec_fn!(abs, $($comp),*);

        fn sign(mut self) -> Self {
            $(self.$comp = self.$comp.signum());*;
            self
        }
    };
}

macro_rules! impl_vecf {
    ($($comp: ident),+) => {
        impl_veci!($($comp),*);

        impl_vec_fn!(floor, $($comp),*);
        impl_vec_fn!(round, $($comp),*);
        impl_vec_fn!(ceil, $($comp),*);
        impl_vec_fn!(fract, $($comp),*);
        impl_vec_fn!(exp, $($comp),*);
        impl_vec_fn!(sin, $($comp),*);
        impl_vec_fn!(cos, $($comp),*);
        impl_vec_fn!(sqrt, $($comp),*);
        impl_vec_fn!(cbrt, $($comp),*);

        fn pow(mut self, p: Self) -> Self {
            $(self.$comp = self.$comp.powf(p.$comp);)*
            self
        }
    };
}

macro_rules! impl_rand {
    ($ty: ty, $($comp: ident),+) => {
        impl Rand for $ty {
            fn rand(mut self) -> Self {
                $(self.$comp = self.$comp.rand();)*
                self
            }

            fn randn_range(mut self, min: Self, max: Self, stdev: f32) -> Self {
                $(self.$comp = self.$comp.randn_range(min.$comp, max.$comp, stdev);)*
                self
            }
        }
    };
}
macro_rules! impl_neg {
    ($ty: ty, $($comp: ident),+) => {
        impl Neg for $ty {
            type Output = Self;
            fn neg(mut self) -> Self {
                $(self.$comp = -self.$comp;)*
                self
            }
        }
    };
}

macro_rules! impl_extra {
    ($ty: ty, $($comp: ident),+) => {
        impl_rand!($ty, $($comp),*);
        impl_neg!($ty, $($comp),*);

        impl ExtraFns for $ty {
            fn lerp(mut self, rhs: Self, k: f32) -> Self {
                $(self.$comp = self.$comp.lerp(rhs.$comp, k);)*
                self
            }

            fn sstep(mut self, e0: Self, e1: Self) -> Self {
                $(self.$comp = self.$comp.sstep(e0.$comp, e1.$comp);)*
                self
            }

            fn step(mut self, rhs: Self) -> Self {
                $(self.$comp = self.$comp.step(rhs.$comp);)*
                self
            }

            fn saturate(self) -> Self {
                self.clamp(Self::ZERO, Self::ONE)
            }

            fn smooth(mut self) -> Self {
                $(self.$comp = self.$comp.smooth();)*
                self
            }
        }
    };
}

pub trait Vectorf: Sized + Copy + Sub<Self, Output = Self> + Mul<Self, Output = Self> {
    fn splat(v: f32) -> Self;
    fn dot(self, rhs: Self) -> f32;
    fn len2(self) -> f32 {
        self.dot(self)
    }
    fn len(self) -> f32;
    fn norm(self) -> Self;
    fn dist2(self, rhs: Self) -> f32 {
        (rhs - self).len2()
    }
    fn dist(self, rhs: Self) -> f32 {
        (rhs - self).len()
    }
    fn sign(self) -> Self;
    fn abs(self) -> Self {
        self * self.sign()
    }
    fn floor(self) -> Self;
    fn round(self) -> Self;
    fn ceil(self) -> Self;
    fn fract(self) -> Self;
    fn exp(self) -> Self;
    fn pow(self, p: Self) -> Self;
    fn angle_between(self, rhs: Self) -> f32;
    fn min(self, rhs: Self) -> Self;
    fn min_elem(self) -> f32;
    fn max(self, rhs: Self) -> Self;
    fn max_elem(self) -> f32;
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn clamp(self, min: Self, max: Self) -> Self;
    fn sqrt(self) -> Self;
    fn cbrt(self) -> Self;
}

pub trait Vectoru: Sized + Copy + Sub<Self, Output = Self> + Mul<Self, Output = Self> {
    fn splat(v: u32) -> Self;
    fn dot(self, rhs: Self) -> u32;
    fn len2(self) -> u32 {
        self.dot(self)
    }
    fn dist2(self, rhs: Self) -> u32 {
        (rhs - self).len2()
    }
    fn min(self, rhs: Self) -> Self;
    fn min_elem(self) -> u32;
    fn max(self, rhs: Self) -> Self;
    fn max_elem(self) -> u32;
    fn clamp(self, min: Self, max: Self) -> Self;
}

pub trait Vectori: Sized + Copy + Sub<Self, Output = Self> + Mul<Self, Output = Self> {
    fn splat(v: i32) -> Self;
    fn dot(self, rhs: Self) -> i32;
    fn len2(self) -> i32 {
        self.dot(self)
    }
    fn dist2(self, rhs: Self) -> i32 {
        (rhs - self).len2()
    }
    fn sign(self) -> Self;
    fn abs(self) -> Self {
        self * self.sign()
    }
    fn min(self, rhs: Self) -> Self;
    fn min_elem(self) -> i32;
    fn max(self, rhs: Self) -> Self;
    fn max_elem(self) -> i32;
    fn clamp(self, min: Self, max: Self) -> Self;
}

#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl_op!(Add, add, +, Vec2, f32, x, y);
impl_op!(Sub, sub, -, Vec2, f32, x, y);
impl_op!(Mul, mul, *, Vec2, f32, x, y);
impl_op!(Div, div, /, Vec2, f32, x, y);
impl_op_assign!(AddAssign, add_assign, +=, Vec2, f32, x, y);
impl_op_assign!(SubAssign, sub_assign, -=, Vec2, f32, x, y);
impl_op_assign!(MulAssign, mul_assign, *=, Vec2, f32, x, y);
impl_op_assign!(DivAssign, div_assign, /=, Vec2, f32, x, y);
impl_extra!(Vec2, x, y);

impl Vec2 {
    const ZERO: Self = Self { x: 0.0, y: 0.0 };
    const ONE: Self = Self { x: 1.0, y: 1.0 };
    const NEG_ONE: Self = Self { x: -1.0, y: -1.0 };
    const X: Self = Self { x: 1.0, y: 0.0 };
    const Y: Self = Self { x: 0.0, y: 1.0 };
    const NEG_X: Self = Self { x: -1.0, y: 0.0 };
    const NEG_Y: Self = Self { x: 0.0, y: -1.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn angle(a: f32) -> Self {
        Self::new(a.cos(), a.sin())
    }

    pub fn cross(self, rhs: Self) -> f32 {
        self.x * rhs.y - self.y * rhs.x
    }
}

impl Noise for Vec2 {
    fn hash(self) -> f32 {
        let ux = (self.x * 141421356.0).to_bits();
        let uy = (self.y * 2718281828.0).to_bits();
        ((ux ^ uy) * 3141592653u32) as f32 / u32::MAX as f32
    }

    fn noise(self) -> f32 {
        let ip = self.floor();
        let u = self.fract().smooth();
        let res = ip
            .hash()
            .lerp(Vec2::new(ip.x + 1.0, ip.y).hash(), u.x)
            .lerp(
                Vec2::new(ip.x, ip.y + 1.0)
                    .hash()
                    .lerp((ip + 1.0).hash(), u.x),
                u.y,
            );
        res * res
    }

    fn voronoise(self, smooth: f32) -> f32 {
        let hash3 = |p: &Self| {
            let q = Vec3::new(
                p.dot(Vec2::new(127.1, 311.7)),
                p.dot(Vec2::new(269.5, 183.3)),
                p.dot(Vec2::new(419.2, 371.9)),
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
                let o = hash3(&(p + &g)) * Vec3::splat(1.0);
                let r = g - &f + Vec2::new(o.x, o.y);
                let d = r.len2();
                let ww = (1.0 - d.sqrt().sstep(0.0, std::f32::consts::SQRT_2)).powf(k);
                va += o.z * ww;
                wt += ww;
            }
        }
        va / wt
    }
}

impl Vectorf for Vec2 {
    impl_vecf!(x, y);

    fn splat(v: f32) -> Self {
        Self::new(v, v)
    }

    fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    fn len(self) -> f32 {
        self.len2().sqrt()
    }

    fn norm(self) -> Self {
        self / self.len()
    }

    fn angle_between(self, rhs: Self) -> f32 {
        self.norm().dot(rhs.norm())
    }

    fn min_elem(self) -> f32 {
        self.x.min(self.y)
    }

    fn max_elem(self) -> f32 {
        self.x.max(self.y)
    }
}

impl From<f32> for Vec2 {
    fn from(value: f32) -> Self {
        Self::splat(value)
    }
}

impl From<(f32, f32)> for Vec2 {
    #[inline(always)]
    fn from(value: (f32, f32)) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl From<u32> for Vec2 {
    fn from(value: u32) -> Self {
        Self::splat(value as f32)
    }
}

impl From<Vec2u> for Vec2 {
    fn from(value: Vec2u) -> Self {
        Self::new(value.x as f32, value.y as f32)
    }
}

#[derive(Clone, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl_op!(Add, add, +, Vec3, f32, x, y, z);
impl_op!(Sub, sub, -, Vec3, f32, x, y, z);
impl_op!(Mul, mul, *, Vec3, f32, x, y, z);
impl_op!(Div, div, /, Vec3, f32, x, y, z);
impl_op_assign!(AddAssign, add_assign, +=, Vec3, f32, x, y, z);
impl_op_assign!(SubAssign, sub_assign, -=, Vec3, f32, x, y, z);
impl_op_assign!(MulAssign, mul_assign, *=, Vec3, f32, x, y, z);
impl_op_assign!(DivAssign, div_assign, /=, Vec3, f32, x, y, z);
impl_extra!(Vec3, x, y, z);

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    pub const NEG_ONE: Self = Self::new(-1.0, -1.0, -1.0);
    pub const X: Self = Self::new(1.0, 0.0, 0.0);
    pub const Y: Self = Self::new(0.0, 1.0, 0.0);
    pub const Z: Self = Self::new(0.0, 0.0, 1.0);
    pub const NEG_X: Self = Self::new(-1.0, 0.0, 0.0);
    pub const NEG_Y: Self = Self::new(0.0, -1.0, 0.0);
    pub const NEG_Z: Self = Self::new(0.0, 0.0, -1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn angle(a: f32, b: f32) -> Self {
        let bc = b.cos();
        Self::new(a.cos() * bc, a.sin() * bc, b.sin())
    }
}

impl Vectorf for Vec3 {
    impl_vecf!(x, y, z);

    fn splat(v: f32) -> Self {
        Self::new(v, v, v)
    }

    fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    fn len(self) -> f32 {
        self.len2().sqrt()
    }

    fn norm(self) -> Self {
        self / self.len()
    }

    fn dist(self, rhs: Self) -> f32 {
        (rhs - self).len()
    }

    fn angle_between(self, rhs: Self) -> f32 {
        self.norm().dot(rhs.norm())
    }

    fn min_elem(self) -> f32 {
        self.x.min(self.y).min(self.z)
    }

    fn max_elem(self) -> f32 {
        self.x.max(self.y).max(self.z)
    }
}

impl From<f32> for Vec3 {
    fn from(value: f32) -> Self {
        Self::splat(value)
    }
}

impl From<Vec3u> for Vec3 {
    fn from(value: Vec3u) -> Self {
        Self::new(value.x as f32, value.y as f32, value.z as f32)
    }
}

#[derive(Clone, Copy)]
pub struct Vec2u {
    pub x: u32,
    pub y: u32,
}

impl_op!(Add, add, +, Vec2u, u32, x, y);
impl_op!(Sub, sub, -, Vec2u, u32, x, y);
impl_op!(Mul, mul, *, Vec2u, u32, x, y);
impl_op!(Div, div, /, Vec2u, u32, x, y);
impl_op_assign!(AddAssign, add_assign, +=, Vec2u, u32, x, y);
impl_op_assign!(SubAssign, sub_assign, -=, Vec2u, u32, x, y);
impl_op_assign!(MulAssign, mul_assign, *=, Vec2u, u32, x, y);
impl_op_assign!(DivAssign, div_assign, /=, Vec2u, u32, x, y);
impl_rand!(Vec2u, x, y);

impl Vec2u {
    pub const ZERO: Self = Self::new(0, 0);
    pub const ONE: Self = Self::new(1, 1);
    pub const X: Self = Self::new(1, 0);
    pub const Y: Self = Self::new(0, 1);

    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

impl Vectoru for Vec2u {
    impl_vecu!(x, y);

    fn splat(v: u32) -> Self {
        Self::new(v, v)
    }

    fn dot(self, rhs: Self) -> u32 {
        self.x * rhs.x + self.y * rhs.y
    }

    fn min_elem(self) -> u32 {
        self.x.min(self.y)
    }

    fn max_elem(self) -> u32 {
        self.x.max(self.y)
    }
}

impl From<u32> for Vec2u {
    fn from(value: u32) -> Self {
        Self::splat(value)
    }
}

impl From<(u32, u32)> for Vec2u {
    fn from(value: (u32, u32)) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl From<Vec2> for Vec2u {
    fn from(value: Vec2) -> Self {
        Self::new(value.x as u32, value.y as u32)
    }
}

#[derive(Clone, Copy)]
pub struct Vec3u {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl_op!(Add, add, +, Vec3u, u32, x, y, z);
impl_op!(Sub, sub, -, Vec3u, u32, x, y, z);
impl_op!(Mul, mul, *, Vec3u, u32, x, y, z);
impl_op!(Div, div, /, Vec3u, u32, x, y, z);
impl_op_assign!(AddAssign, add_assign, +=, Vec3u, u32, x, y, z);
impl_op_assign!(SubAssign, sub_assign, -=, Vec3u, u32, x, y, z);
impl_op_assign!(MulAssign, mul_assign, *=, Vec3u, u32, x, y, z);
impl_op_assign!(DivAssign, div_assign, /=, Vec3u, u32, x, y, z);
impl_rand!(Vec3u, x, y);

impl Vec3u {
    pub const ZERO: Self = Self::new(0, 0, 0);
    pub const ONE: Self = Self::new(1, 1, 1);
    pub const X: Self = Self::new(1, 0, 0);
    pub const Y: Self = Self::new(0, 1, 0);
    pub const Z: Self = Self::new(0, 0, 1);

    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }
}

impl Vectoru for Vec3u {
    impl_vecu!(x, y);

    fn splat(v: u32) -> Self {
        Self::new(v, v, v)
    }

    fn dot(self, rhs: Self) -> u32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    fn min_elem(self) -> u32 {
        self.x.min(self.y.min(self.z))
    }

    fn max_elem(self) -> u32 {
        self.x.max(self.y.max(self.z))
    }
}

impl From<u32> for Vec3u {
    fn from(value: u32) -> Self {
        Self::splat(value)
    }
}

#[derive(Clone, Copy)]
pub struct Vec2i {
    pub x: i32,
    pub y: i32,
}

impl_op!(Add, add, +, Vec2i, i32, x, y);
impl_op!(Sub, sub, -, Vec2i, i32, x, y);
impl_op!(Mul, mul, *, Vec2i, i32, x, y);
impl_op!(Div, div, /, Vec2i, i32, x, y);
impl_op_assign!(AddAssign, add_assign, +=, Vec2i, i32, x, y);
impl_op_assign!(SubAssign, sub_assign, -=, Vec2i, i32, x, y);
impl_op_assign!(MulAssign, mul_assign, *=, Vec2i, i32, x, y);
impl_op_assign!(DivAssign, div_assign, /=, Vec2i, i32, x, y);
impl_rand!(Vec2i, x, y);
impl_neg!(Vec2i, x, y);

impl Vec2i {
    pub const ZERO: Self = Self::new(0, 0);
    pub const ONE: Self = Self::new(1, 1);
    pub const NEG_ONE: Self = Self::new(-1, -1);
    pub const X: Self = Self::new(1, 0);
    pub const Y: Self = Self::new(0, 1);
    pub const NEG_X: Self = Self::new(-1, 0);
    pub const NEG_Y: Self = Self::new(0, -1);

    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl Vectori for Vec2i {
    impl_veci!(x, y);

    fn splat(v: i32) -> Self {
        Self::new(v, v)
    }

    fn dot(self, rhs: Self) -> i32 {
        self.x * rhs.x + self.y * rhs.y
    }

    fn min_elem(self) -> i32 {
        self.x.min(self.y)
    }

    fn max_elem(self) -> i32 {
        self.x.max(self.y)
    }
}

impl From<i32> for Vec2i {
    fn from(value: i32) -> Self {
        Self::splat(value)
    }
}
