#![allow(unused)]
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Neg, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};

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
        if b < self { 0.0 } else { 1.0 }
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
    ($trait: ident, $method: ident, $wrap_method: ident, $ty: ty, $comp_ty: ty, $($comp: ident),+) => {
        impl $trait<&$ty> for $ty {
            type Output = Self;
            fn $method(self, rhs: &Self) -> Self {
                Self::Output {
                    $($comp: self.$comp.$wrap_method(rhs.$comp)),*
                }
            }
        }

        impl $trait<$ty> for $ty {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self {
                Self::Output {
                    $($comp: self.$comp.$wrap_method(rhs.$comp)),*
                }
            }
        }

        impl $trait<$comp_ty> for $ty {
            type Output = Self;
            fn $method(self, rhs: $comp_ty) -> Self {
                Self::Output {
                    $($comp: self.$comp.$wrap_method(rhs)),*
                }
            }
        }

        impl $trait<$ty> for $comp_ty {
            type Output = $ty;
            fn $method(self, rhs: $ty) -> Self::Output {
                Self::Output {
                    $($comp: self.$wrap_method(rhs.$comp)),*
                }
            }
        }
    };
}

macro_rules! impl_op_assign {
    ($trait: ident, $method: ident, $ty: ty, $comp_ty: ty, $($comp: ident),+) => {
        impl $trait<&$ty> for $ty {
            fn $method(&mut self, rhs: &Self) {
                $(self.$comp.$method(rhs.$comp);)*
            }
        }

        impl $trait<$ty> for $ty {
            fn $method(&mut self, rhs: Self) {
                $(self.$comp.$method(rhs.$comp);)*
            }
        }

        impl $trait<$comp_ty> for $ty {
            fn $method(&mut self, rhs: $comp_ty) {
                $(self.$comp.$method(rhs);)*
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

macro_rules! fold {
    ($fn: ident, $self: ident, $field1: ident, $($fields: ident),*) => {
        $self.$field1.$fn(fold!($fn, $self, $($fields),+))
    };
    ($fn: ident, $self: ident, $field: ident) => {
        $self.$field
    };
}

macro_rules! impl_splat {
    ($comp_ty: ty, $x: ident, $y: ident, $z: ident, $w: ident) => {
        fn splat(x: $comp_ty) -> Self {
            Self::new(x, x, x, x)
        }
    };
    ($comp_ty: ty, $x: ident, $y: ident, $z: ident) => {
        fn splat(x: $comp_ty) -> Self {
            Self::new(x, x, x)
        }
    };
    ($comp_ty: ty, $x: ident, $y: ident) => {
        fn splat(x: $comp_ty) -> Self {
            Self::new(x, x)
        }
    };
}

macro_rules! impl_tuple_helper {
    ($ty: ty, $tuple_ty: ty) => {
        impl From<$tuple_ty> for $ty {
            fn from(value: $tuple_ty) -> Self {
                unsafe { std::mem::transmute(value) }
            }
        }
    };
}

macro_rules! impl_from_tuple {
    ($ty: ty, $comp_ty: ty, $x: ident, $y: ident, $z: ident, $w: ident) => {
        impl_tuple_helper!($ty, (u32, u32, u32, u32));
        impl_tuple_helper!($ty, (i32, i32, i32, i32));
        impl_tuple_helper!($ty, (f32, f32, f32, f32));
    };
    ($ty: ty, $comp_ty: ty, $x: ident, $y: ident, $z: ident) => {
        impl_tuple_helper!($ty, (u32, u32, u32));
        impl_tuple_helper!($ty, (i32, i32, i32));
        impl_tuple_helper!($ty, (f32, f32, f32));
    };
    ($ty: ty, $comp_ty: ty, $x: ident, $y: ident) => {
        impl_tuple_helper!($ty, (u32, u32));
        impl_tuple_helper!($ty, (i32, i32));
        impl_tuple_helper!($ty, (f32, f32));
    };
}

macro_rules! impl_vecu {
    (+, $ty: ty, $comp_ty: ty, $first_comp: ident, $($comp: ident),+) => {
        impl Vectoru for $ty {
            impl_vecu!($comp_ty, $first_comp, $($comp),+);
        }
    };
    ($comp_ty: ty, $first_comp: ident, $($comp: ident),+) => {
        fn max(mut self, rhs: Self) -> Self {
            self.$first_comp = self.$first_comp.max(rhs.$first_comp);
            $(self.$comp = self.$comp.max(rhs.$comp);)*
            self
        }

        fn min(mut self, rhs: Self) -> Self {
            self.$first_comp = self.$first_comp.min(rhs.$first_comp);
            $(self.$comp = self.$comp.min(rhs.$comp);)*
            self
        }

        fn max_elem(self) -> $comp_ty {
            fold!(max, self, $first_comp, $($comp),*)
        }

        fn min_elem(self) -> $comp_ty {
            fold!(min, self, $first_comp, $($comp),*)
        }

        fn dot(self, rhs: Self) -> $comp_ty {
            let mut result = self.$first_comp * rhs.$first_comp;
            $(result += self.$comp * rhs.$comp;)*
            result
        }

        fn clamp(mut self, min: Self, max: Self) -> Self {
            self.$first_comp = self.$first_comp.clamp(min.$first_comp, max.$first_comp);
            $(self.$comp = self.$comp.clamp(min.$comp, max.$comp);)*
            self
        }

        fn rem(mut self, div: Self) -> Self {
            self.$first_comp = self.$first_comp % div.$first_comp;
            $(self.$comp = self.$comp % div.$comp;)*
            self
        }

        impl_splat!($comp_ty, $first_comp, $($comp),*);
    };
}

macro_rules! impl_veci {
    (+, $ty: ty, $comp_ty: ty, $first_comp: ident, $($comp: ident),+) => {
        impl Vectori for $ty {
            impl_veci!($comp_ty, $first_comp, $($comp),+);
        }
    };
    ($comp_ty: ty, $first_comp: ident, $($comp: ident),+) => {
        impl_vecu!($comp_ty, $first_comp, $($comp),*);

        impl_vec_fn!(abs, $first_comp, $($comp),*);

        fn sign(mut self) -> Self {
            self.$first_comp = self.$first_comp.signum();
            $(self.$comp = self.$comp.signum());*;
            self
        }

        fn rem_euclid(mut self, div: Self) -> Self {
            self.$first_comp = self.$first_comp.rem_euclid(div.$first_comp);
            $(self.$comp = self.$comp.rem_euclid(div.$comp);)*
            self
        }
    };
}

macro_rules! impl_vecf {
    ($ty: ty, $vecu: ty, $first_comp: ident, $($comp: ident),+) => {
        impl Vectorf for $ty {
            type Vecu = $vecu;

            impl_veci!(f32, $first_comp, $($comp),*);

            impl_vec_fn!(floor, $first_comp, $($comp),*);
            impl_vec_fn!(round, $first_comp, $($comp),*);
            impl_vec_fn!(ceil, $first_comp, $($comp),*);
            impl_vec_fn!(exp, $first_comp, $($comp),*);
            impl_vec_fn!(sin, $first_comp, $($comp),*);
            impl_vec_fn!(cos, $first_comp, $($comp),*);
            impl_vec_fn!(sqrt, $first_comp, $($comp),*);
            impl_vec_fn!(cbrt, $first_comp, $($comp),*);

            fn len(self) -> f32 {
                self.len2().sqrt()
            }

            fn norm(self) -> Self {
                self / self.len()
            }

            fn dist(self, rhs: Self) -> f32 {
                (rhs - self).len()
            }

            fn pow(mut self, p: Self) -> Self {
                self.$first_comp = self.$first_comp.powf(p.$first_comp);
                $(self.$comp = self.$comp.powf(p.$comp);)*
                self
            }

            fn fract(mut self) -> Self {
                self.$first_comp = self.$first_comp - self.$first_comp.floor();
                $(self.$comp = self.$comp - self.$comp.floor();)*
                self
            }

            fn angle_between(self, rhs: Self) -> f32 {
                self.norm().dot(rhs.norm()).acos()
            }

            fn to_bits(self) -> Self::Vecu {
                <$vecu>::new(self.$first_comp.to_bits(), $(self.$comp.to_bits()),*)
            }
        }

        impl From<$ty> for $vecu {
            fn from(v: $ty) -> $vecu {
                <$vecu>::new(v.$first_comp as u32, $(v.$comp as u32),*)
            }
        }

        impl From<$vecu> for $ty {
            fn from(v: $vecu) -> Self {
                <$ty>::new(v.$first_comp as f32, $(v.$comp as f32),*)
            }
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

macro_rules! impl_vecf_extra {
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

macro_rules! impl_ops {
    ($ty: ty, u32, $($comp: ident),+) => {
        impl_op!(Add, add, wrapping_add, $ty, $comp_ty, $($comp),+);
        impl_op!(Sub, sub, wrapping_sub, $ty, $comp_ty, $($comp),+);
        impl_op!(Mul, mul, wrapping_mul, $ty, $comp_ty, $($comp),+);
        impl_op!(Div, div, wrapping_div, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(AddAssign, add_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(SubAssign, sub_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(MulAssign, mul_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(DivAssign, div_assign, $ty, $comp_ty, $($comp),+);
    };
    ($ty: ty, $comp_ty: ty, $($comp: ident),+) => {
        impl_op!(Add, add, add, $ty, $comp_ty, $($comp),+);
        impl_op!(Sub, sub, sub, $ty, $comp_ty, $($comp),+);
        impl_op!(Mul, mul, mul, $ty, $comp_ty, $($comp),+);
        impl_op!(Div, div, div, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(AddAssign, add_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(SubAssign, sub_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(MulAssign, mul_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(DivAssign, div_assign, $ty, $comp_ty, $($comp),+);

    };
}

macro_rules! impl_bitops {
    ($ty: ty, $comp_ty: ty, $($comp: ident),+) => {
        impl_op!(BitOr, bitor, bitor, $ty, $comp_ty, $($comp),+);
        impl_op!(BitAnd, bitand, bitand, $ty, $comp_ty, $($comp),+);
        impl_op!(BitXor, bitxor, bitxor, $ty, $comp_ty, $($comp),+);
        impl_op!(Shl, shl, shl, $ty, $comp_ty, $($comp),+);
        impl_op!(Shr, shr, shr, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(BitOrAssign, bitor_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(BitAndAssign, bitand_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(BitXorAssign, bitxor_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(ShlAssign, shl_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(ShrAssign, shr_assign, $ty, $comp_ty, $($comp),+);
    };
}

macro_rules! def_const {
    ($name: ident, $cty: ty, $($comp: expr),+) => {
        pub const $name: Self = Self::new($($comp as $cty),*);
    };
}

macro_rules! def_vec_neg_consts {
    ($ty: ty, $cty: ty, $x: ident, $y: ident, $z: ident, $w: ident) => {
        def_const!(NEG_ONE, $cty, -1, -1, -1, -1);
        def_const!(NEG_X, $cty, -1, 0, 0, 0);
        def_const!(NEG_Y, $cty, 0, -1, 0, 0);
        def_const!(NEG_Z, $cty, 0, 0, -1, 0);
        def_const!(NEG_W, $cty, 0, 0, 0, -1);
    };
    ($ty: ty, $cty: ty, $x: ident, $y: ident, $z: ident) => {
        def_const!(NEG_ONE, $cty, -1, -1, -1);
        def_const!(NEG_X, $cty, -1, 0, 0);
        def_const!(NEG_Y, $cty, 0, -1, 0);
        def_const!(NEG_Z, $cty, 0, 0, -1);
    };
    ($ty: ty, $cty: ty, $x: ident, $y: ident) => {
        def_const!(NEG_ONE, $cty, -1, -1);
        def_const!(NEG_X, $cty, -1, 0);
        def_const!(NEG_Y, $cty, 0, -1);
        def_const!(NEG_Z, $cty, 0, 0);
    };
}

macro_rules! def_vec_consts {
    (-, $ty: ty, i32, $($comp: ident),+) => {
        def_vec_neg_consts!($ty, i32, $($comp),*);
        def_vec_consts!(+, $ty, i32, $($comp),*);
    };
    (-, $ty: ty, f32, $($comp: ident),+) => {
        def_vec_neg_consts!($ty, f32, $($comp),*);
        def_vec_consts!(+, $ty, f32, $($comp),*);
    };
    (-, $ty: ty, $cty: ty, $($comp: ident),+) => {
        def_vec_consts!(+, $ty, $cty, $($comp),*);
    };
    (+, $ty: ty, $cty: ty, $x: ident, $y: ident, $z: ident, $w: ident) => {
        def_const!(ZERO, $cty, 0, 0, 0, 0);
        def_const!(ONE, $cty, 1, 1, 1, 1);
        def_const!(
            MIN,
            $cty,
            <$cty>::MIN,
            <$cty>::MIN,
            <$cty>::MIN,
            <$cty>::MIN
        );
        def_const!(
            MAX,
            $cty,
            <$cty>::MAX,
            <$cty>::MAX,
            <$cty>::MAX,
            <$cty>::MAX
        );
        def_const!(X, $cty, 1, 0, 0, 0);
        def_const!(Y, $cty, 0, 1, 0, 0);
        def_const!(Z, $cty, 0, 0, 1, 0);
        def_const!(W, $cty, 0, 0, 0, 1);
    };
    (+, $ty: ty, $cty: ty, $x: ident, $y: ident, $z: ident) => {
        def_const!(ZERO, $cty, 0, 0, 0);
        def_const!(ONE, $cty, 1, 1, 1);
        def_const!(MIN, $cty, <$cty>::MIN, <$cty>::MIN, <$cty>::MIN);
        def_const!(MAX, $cty, <$cty>::MAX, <$cty>::MAX, <$cty>::MAX);
        def_const!(X, $cty, 1, 0, 0);
        def_const!(Y, $cty, 0, 1, 0);
        def_const!(Z, $cty, 0, 0, 1);
    };
    (+, $ty: ty, $cty: ty, $x: ident, $y: ident) => {
        def_const!(ZERO, $cty, 0, 0);
        def_const!(ONE, $cty, 1, 1);
        def_const!(MIN, $cty, <$cty>::MIN, <$cty>::MIN);
        def_const!(MAX, $cty, <$cty>::MAX, <$cty>::MAX);
        def_const!(X, $cty, 1, 0);
        def_const!(Y, $cty, 0, 1);
    };
}

macro_rules! def_vec {
    ($ty: ident, $comp_ty: ty, $($comp: ident),+) => {
        #[derive(Clone, Copy, Debug)]
        pub struct $ty {
            $(pub $comp: $comp_ty),*
        }
        impl_ops!($ty, $comp_ty, $($comp),*);

        impl From<u32> for $ty {
            fn from(value: u32) -> Self {
                Self::splat(value as $comp_ty)
            }
        }

        impl From<i32> for $ty {
            fn from(value: i32) -> Self {
                Self::splat(value as $comp_ty)
            }
        }

        impl From<f32> for $ty {
            fn from(value: f32) -> Self {
                Self::splat(value as $comp_ty)
            }
        }

        impl_from_tuple!($ty, $comp_ty, $($comp),+);

        impl $ty {
            def_vec_consts!(-, $ty, $comp_ty, $($comp),+);

            pub const fn new($($comp: $comp_ty),+) -> Self {
                Self { $($comp),* }
            }
        }
    };
}

macro_rules! def_vecu {
    (+, $ty: ident, $comp_ty: ty, $($comps: ident),+) => {
        def_vec!($ty, $comp_ty, $($comps),*);
        impl_bitops!($ty, $comp_ty, $($comps),*);
        impl_rand!($ty, $($comps),*);
    };
    ($ty: ident, $($comps: ident),+) => {
        def_vecu!(+, $ty, u32, $($comps),*);
        impl_vecu!(+, $ty, u32, $($comps),*);
    };
}

macro_rules! def_veci {
    ($ty: ident, $($comps: ident),+) => {
        def_vecu!(+, $ty, i32, $($comps),*);
        impl_veci!(+, $ty, i32, $($comps),*);
        impl_neg!($ty, $($comps),*);
    };
}

macro_rules! def_vecf {
    ($ty: ident, $vecu: ty, $($comps: ident),+) => {
        def_vec!($ty, f32, $($comps),*);
        impl_vecf_extra!($ty, $($comps),*);
        impl_vecf!($ty, $vecu, $($comps),*);
    };
}

#[macro_export]
macro_rules! swiz {
    ($swiz_ty: ty, $name: ident. $($comp: ident)+) => {
        <$swiz_ty>::new($($name.$comp),+)
    }
}

pub trait Vectorf: Sized + Copy + Sub<Self, Output = Self> + Mul<Self, Output = Self> {
    type Vecu;
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
    fn rem(self, div: Self) -> Self;
    fn rem_euclid(self, div: Self) -> Self;
    fn to_bits(self) -> Self::Vecu;
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
    fn rem(self, div: Self) -> Self;
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
    fn rem(self, div: Self) -> Self;
    fn rem_euclid(self, div: Self) -> Self;
}

def_vecf!(Vec2, Vec2u, x, y);
def_vecf!(Vec3, Vec3u, x, y, z);
def_vecf!(Vec4, Vec4u, x, y, z, w);

def_vecu!(Vec2u, x, y);
def_vecu!(Vec3u, x, y, z);
def_vecu!(Vec4u, x, y, z, w);

def_veci!(Vec2i, x, y);
def_veci!(Vec3i, x, y, z);
def_veci!(Vec4i, x, y, z, w);

impl Vec2 {
    pub fn angle(a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Self::new(s, c)
    }

    pub fn rotate(self, a: f32) -> Self {
        let (s, c) = a.sin_cos();
        c * self + s * Vec2::new(-self.y, self.x)
    }

    pub fn cross(self, rhs: Self) -> f32 {
        self.x * rhs.y - self.y * rhs.x
    }
}

impl Vec3 {
    pub fn angle(a: f32, b: f32) -> Self {
        let bc = b.cos();
        Self::new(a.cos() * bc, a.sin() * bc, b.sin())
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }
}
