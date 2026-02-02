#![allow(dead_code)]
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Neg, Not, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};

// ============================================================================
// Common Operator Macros
// ============================================================================

macro_rules! impl_arith_ops {
    ($vec:ident, $scalar:ty, [$($i:tt),+]) => {
        // Vec op Vec
        impl Add for $vec { type Output = Self; #[inline] fn add(self, rhs: Self) -> Self { Self([$(self.0[$i] + rhs.0[$i]),+]) } }
        impl Sub for $vec { type Output = Self; #[inline] fn sub(self, rhs: Self) -> Self { Self([$(self.0[$i] - rhs.0[$i]),+]) } }
        impl Mul for $vec { type Output = Self; #[inline] fn mul(self, rhs: Self) -> Self { Self([$(self.0[$i] * rhs.0[$i]),+]) } }
        impl Div for $vec { type Output = Self; #[inline] fn div(self, rhs: Self) -> Self { Self([$(self.0[$i] / rhs.0[$i]),+]) } }
        impl Rem for $vec { type Output = Self; #[inline] fn rem(self, rhs: Self) -> Self { Self([$(self.0[$i] % rhs.0[$i]),+]) } }

        // Vec op scalar
        impl Mul<$scalar> for $vec { type Output = Self; #[inline] fn mul(self, rhs: $scalar) -> Self { Self([$(self.0[$i] * rhs),+]) } }
        impl Div<$scalar> for $vec { type Output = Self; #[inline] fn div(self, rhs: $scalar) -> Self { Self([$(self.0[$i] / rhs),+]) } }
        impl Add<$scalar> for $vec { type Output = Self; #[inline] fn add(self, rhs: $scalar) -> Self { Self([$(self.0[$i] + rhs),+]) } }
        impl Sub<$scalar> for $vec { type Output = Self; #[inline] fn sub(self, rhs: $scalar) -> Self { Self([$(self.0[$i] - rhs),+]) } }
        impl Rem<$scalar> for $vec { type Output = Self; #[inline] fn rem(self, rhs: $scalar) -> Self { Self([$(self.0[$i] % rhs),+]) } }

        // Assign ops
        impl AddAssign for $vec { #[inline] fn add_assign(&mut self, rhs: Self) { $(self.0[$i] += rhs.0[$i];)+ } }
        impl SubAssign for $vec { #[inline] fn sub_assign(&mut self, rhs: Self) { $(self.0[$i] -= rhs.0[$i];)+ } }
        impl MulAssign for $vec { #[inline] fn mul_assign(&mut self, rhs: Self) { $(self.0[$i] *= rhs.0[$i];)+ } }
        impl DivAssign for $vec { #[inline] fn div_assign(&mut self, rhs: Self) { $(self.0[$i] /= rhs.0[$i];)+ } }
        impl RemAssign for $vec { #[inline] fn rem_assign(&mut self, rhs: Self) { $(self.0[$i] %= rhs.0[$i];)+ } }

        impl AddAssign<$scalar> for $vec { #[inline] fn add_assign(&mut self, rhs: $scalar) { $(self.0[$i] += rhs;)+ } }
        impl SubAssign<$scalar> for $vec { #[inline] fn sub_assign(&mut self, rhs: $scalar) { $(self.0[$i] -= rhs;)+ } }
        impl MulAssign<$scalar> for $vec { #[inline] fn mul_assign(&mut self, rhs: $scalar) { $(self.0[$i] *= rhs;)+ } }
        impl DivAssign<$scalar> for $vec { #[inline] fn div_assign(&mut self, rhs: $scalar) { $(self.0[$i] /= rhs;)+ } }
        impl RemAssign<$scalar> for $vec { #[inline] fn rem_assign(&mut self, rhs: $scalar) { $(self.0[$i] %= rhs;)+ } }

        impl Add<$vec> for $scalar { type Output = $vec; #[inline] fn add(self, rhs: $vec) -> $vec { rhs + self } }
        impl Mul<$vec> for $scalar { type Output = $vec; #[inline] fn mul(self, rhs: $vec) -> $vec { rhs * self } }
        impl Sub<$vec> for $scalar { type Output = $vec; #[inline] fn sub(self, rhs: $vec) -> $vec { $vec([$(self - rhs.0[$i]),+]) } }
        impl Div<$vec> for $scalar { type Output = $vec; #[inline] fn div(self, rhs: $vec) -> $vec { $vec([$(self / rhs.0[$i]),+]) } }
        impl Rem<$vec> for $scalar { type Output = $vec; #[inline] fn rem(self, rhs: $vec) -> $vec { $vec([$(self % rhs.0[$i]),+]) } }
    };
}

macro_rules! impl_neg {
    ($vec:ident, [$($i:tt),+]) => {
        impl Neg for $vec { type Output = Self; #[inline] fn neg(self) -> Self { Self([$(-self.0[$i]),+]) } }
    };
}

macro_rules! impl_bit_ops_i32 {
    ($vec:ident, [$($i:tt),+]) => {
        impl BitAnd for $vec { type Output = Self; #[inline] fn bitand(self, rhs: Self) -> Self { Self([$(self.0[$i] & rhs.0[$i]),+]) } }
        impl BitOr for $vec { type Output = Self; #[inline] fn bitor(self, rhs: Self) -> Self { Self([$(self.0[$i] | rhs.0[$i]),+]) } }
        impl BitXor for $vec { type Output = Self; #[inline] fn bitxor(self, rhs: Self) -> Self { Self([$(self.0[$i] ^ rhs.0[$i]),+]) } }
        impl Not for $vec { type Output = Self; #[inline] fn not(self) -> Self { Self([$(! self.0[$i]),+]) } }
        impl Shl<i32> for $vec { type Output = Self; #[inline] fn shl(self, rhs: i32) -> Self { Self([$(self.0[$i] << rhs),+]) } }
        impl Shr<i32> for $vec { type Output = Self; #[inline] fn shr(self, rhs: i32) -> Self { Self([$(self.0[$i] >> rhs),+]) } }

        impl BitAndAssign for $vec { #[inline] fn bitand_assign(&mut self, rhs: Self) { $(self.0[$i] &= rhs.0[$i];)+ } }
        impl BitOrAssign for $vec { #[inline] fn bitor_assign(&mut self, rhs: Self) { $(self.0[$i] |= rhs.0[$i];)+ } }
        impl BitXorAssign for $vec { #[inline] fn bitxor_assign(&mut self, rhs: Self) { $(self.0[$i] ^= rhs.0[$i];)+ } }
        impl ShlAssign<i32> for $vec { #[inline] fn shl_assign(&mut self, rhs: i32) { $(self.0[$i] <<= rhs;)+ } }
        impl ShrAssign<i32> for $vec { #[inline] fn shr_assign(&mut self, rhs: i32) { $(self.0[$i] >>= rhs;)+ } }
    };
}

macro_rules! impl_bit_ops_u32 {
    ($vec:ident, [$($i:tt),+]) => {
        impl BitAnd for $vec { type Output = Self; #[inline] fn bitand(self, rhs: Self) -> Self { Self([$(self.0[$i] & rhs.0[$i]),+]) } }
        impl BitOr for $vec { type Output = Self; #[inline] fn bitor(self, rhs: Self) -> Self { Self([$(self.0[$i] | rhs.0[$i]),+]) } }
        impl BitXor for $vec { type Output = Self; #[inline] fn bitxor(self, rhs: Self) -> Self { Self([$(self.0[$i] ^ rhs.0[$i]),+]) } }
        impl Not for $vec { type Output = Self; #[inline] fn not(self) -> Self { Self([$(! self.0[$i]),+]) } }
        impl Shl<u32> for $vec { type Output = Self; #[inline] fn shl(self, rhs: u32) -> Self { Self([$(self.0[$i] << rhs),+]) } }
        impl Shr<u32> for $vec { type Output = Self; #[inline] fn shr(self, rhs: u32) -> Self { Self([$(self.0[$i] >> rhs),+]) } }

        impl BitAndAssign for $vec { #[inline] fn bitand_assign(&mut self, rhs: Self) { $(self.0[$i] &= rhs.0[$i];)+ } }
        impl BitOrAssign for $vec { #[inline] fn bitor_assign(&mut self, rhs: Self) { $(self.0[$i] |= rhs.0[$i];)+ } }
        impl BitXorAssign for $vec { #[inline] fn bitxor_assign(&mut self, rhs: Self) { $(self.0[$i] ^= rhs.0[$i];)+ } }
        impl ShlAssign<u32> for $vec { #[inline] fn shl_assign(&mut self, rhs: u32) { $(self.0[$i] <<= rhs;)+ } }
        impl ShrAssign<u32> for $vec { #[inline] fn shr_assign(&mut self, rhs: u32) { $(self.0[$i] >>= rhs;)+ } }
    };
}

macro_rules! impl_from_tuple2 {
    ($vec:ident, $scalar:ty) => {
        impl From<($scalar, $scalar)> for $vec {
            #[inline]
            fn from(t: ($scalar, $scalar)) -> Self {
                Self([t.0, t.1])
            }
        }
    };
}

macro_rules! impl_from_tuple3 {
    ($vec:ident, $scalar:ty) => {
        impl From<($scalar, $scalar, $scalar)> for $vec {
            #[inline]
            fn from(t: ($scalar, $scalar, $scalar)) -> Self {
                Self([t.0, t.1, t.2])
            }
        }
    };
}

macro_rules! impl_from_tuple4 {
    ($vec:ident, $scalar:ty) => {
        impl From<($scalar, $scalar, $scalar, $scalar)> for $vec {
            #[inline]
            fn from(t: ($scalar, $scalar, $scalar, $scalar)) -> Self {
                Self([t.0, t.1, t.2, t.3])
            }
        }
    };
}

// ============================================================================
// Vec Definition Macros
// ============================================================================

macro_rules! define_vecf {
    ($vec:ident, $vecu:ident, $veci:ident, $n:literal, [$($i:tt),+], [$($comp:ident),+], $tuple_macro:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $vec([f32; $n]);

        impl_arith_ops!($vec, f32, [$($i),+]);
        impl_neg!($vec, [$($i),+]);
        $tuple_macro!($vec, f32);
        impl $vec {
            pub const ZERO: $vec = $vec::splat(0.0);
            pub const ONE: $vec = $vec::splat(1.0);
            #[inline] pub const fn new($($comp: f32),+) -> Self { Self([$($comp),+]) }
            #[inline] pub const fn splat(v: f32) -> Self { Self([$({ let _ = stringify!($comp); v }),+]) }
            $(#[inline] pub fn $comp(&self) -> f32 { self.0[${index()}] })+
            #[inline] pub fn dot(self, rhs: Self) -> f32 { 0.0 $(+ self.0[$i] * rhs.0[$i])+ }
            #[inline] pub fn len2(self) -> f32 { self.dot(self) }
            #[inline] pub fn len(self) -> f32 { self.len2().sqrt() }
            #[inline] pub fn dist2(self, rhs: Self) -> f32 { (rhs - self).len2() }
            #[inline] pub fn dist(self, rhs: Self) -> f32 { (rhs - self).len() }
            #[inline] pub fn norm(self) -> Self { let l = self.len(); if l > 0.0 { self / l } else { self } }
            #[inline] pub fn sign(self) -> Self { Self([$(self.0[$i].signum()),+]) }
            #[inline] pub fn abs(self) -> Self { Self([$(self.0[$i].abs()),+]) }
            #[inline] pub fn floor(self) -> Self { Self([$(self.0[$i].floor()),+]) }
            #[inline] pub fn round(self) -> Self { Self([$(self.0[$i].round()),+]) }
            #[inline] pub fn ceil(self) -> Self { Self([$(self.0[$i].ceil()),+]) }
            #[inline] pub fn fract(self) -> Self { Self([$(self.0[$i].fract()),+]) }
            #[inline] pub fn exp(self) -> Self { Self([$(self.0[$i].exp()),+]) }
            #[inline] pub fn pow(self, p: Self) -> Self { Self([$(self.0[$i].powf(p.0[$i])),+]) }
            #[inline] pub fn sqrt(self) -> Self { Self([$(self.0[$i].sqrt()),+]) }
            #[inline] pub fn cbrt(self) -> Self { Self([$(self.0[$i].cbrt()),+]) }
            #[inline] pub fn sin(self) -> Self { Self([$(self.0[$i].sin()),+]) }
            #[inline] pub fn cos(self) -> Self { Self([$(self.0[$i].cos()),+]) }
            #[inline] pub fn min(self, rhs: Self) -> Self { Self([$(self.0[$i].min(rhs.0[$i])),+]) }
            #[inline] pub fn max(self, rhs: Self) -> Self { Self([$(self.0[$i].max(rhs.0[$i])),+]) }
            #[inline] pub fn min_elem(self) -> f32 { let mut m = self.0[0]; $(m = m.min(self.0[$i]);)+ m }
            #[inline] pub fn max_elem(self) -> f32 { let mut m = self.0[0]; $(m = m.max(self.0[$i]);)+ m }
            #[inline] pub fn clamp(self, min: Self, max: Self) -> Self { Self([$(self.0[$i].clamp(min.0[$i], max.0[$i])),+]) }
            #[inline] pub fn clamp_scalar(self, min: f32, max: f32) -> Self { Self([$(self.0[$i].clamp(min, max)),+]) }
            #[inline] pub fn saturate(self) -> Self { Self([$(self.0[$i].clamp(0.0, 1.0)),+]) }
            #[inline] pub fn rem_euclid(self, div: Self) -> Self { Self([$(self.0[$i].rem_euclid(div.0[$i])),+]) }
            #[inline] pub fn angle_between(self, rhs: Self) -> f32 { (self.dot(rhs) / (self.len() * rhs.len())).acos() }
            #[inline] pub fn to_bits(self) -> $vecu { $vecu([$(self.0[$i].to_bits()),+]) }
            #[inline] pub fn to_int(self) -> $veci { $veci([$(self.0[$i] as i32),+]) }
            #[inline] pub fn to_uint(self) -> $vecu { $vecu([$(self.0[$i] as u32),+]) }
        }
    };
}

macro_rules! define_veci {
    ($vec:ident, $vecf:ident, $vecu:ident, $n:literal, [$($i:tt),+], [$($comp:ident),+], $tuple_macro:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $vec([i32; $n]);

        impl_arith_ops!($vec, i32, [$($i),+]);
        impl_neg!($vec, [$($i),+]);
        impl_bit_ops_i32!($vec, [$($i),+]);
        $tuple_macro!($vec, i32);

        impl $vec {
            pub const ZERO: $vec = $vec::splat(0);
            pub const ONE: $vec = $vec::splat(1);
            #[inline] pub const fn new($($comp: i32),+) -> Self { Self([$($comp),+]) }
            #[inline] pub const fn splat(v: i32) -> Self { Self([$({ let _ = stringify!($comp); v }),+]) }
            $(#[inline] pub fn $comp(&self) -> i32 { self.0[${index()}] })+
            #[inline] pub fn abs(self) -> Self { Self([$(self.0[$i].abs()),+]) }
            #[inline] pub fn signum(self) -> Self { Self([$(self.0[$i].signum()),+]) }
            #[inline] pub fn min(self, rhs: Self) -> Self { Self([$(self.0[$i].min(rhs.0[$i])),+]) }
            #[inline] pub fn max(self, rhs: Self) -> Self { Self([$(self.0[$i].max(rhs.0[$i])),+]) }
            #[inline] pub fn min_elem(self) -> i32 { let mut m = self.0[0]; $(m = m.min(self.0[$i]);)+ m }
            #[inline] pub fn max_elem(self) -> i32 { let mut m = self.0[0]; $(m = m.max(self.0[$i]);)+ m }
            #[inline] pub fn clamp(self, min: Self, max: Self) -> Self { Self([$(self.0[$i].clamp(min.0[$i], max.0[$i])),+]) }
            #[inline] pub fn to_float(self) -> $vecf { $vecf([$(self.0[$i] as f32),+]) }
            #[inline] pub fn to_uint(self) -> $vecu { $vecu([$(self.0[$i] as u32),+]) }
            #[inline] pub fn to_bits(self) -> $vecu { $vecu([$(self.0[$i] as u32),+]) }
        }
    };
}

macro_rules! define_vecu {
    ($vec:ident, $vecf:ident, $veci:ident, $n:literal, [$($i:tt),+], [$($comp:ident),+], $tuple_macro:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct $vec([u32; $n]);

        impl_arith_ops!($vec, u32, [$($i),+]);
        impl_bit_ops_u32!($vec, [$($i),+]);
        $tuple_macro!($vec, u32);

        impl $vec {
            pub const ZERO: $vec = $vec::splat(0);
            pub const ONE: $vec = $vec::splat(1);
            #[inline] pub const fn new($($comp: u32),+) -> Self { Self([$($comp),+]) }
            #[inline] pub const fn splat(v: u32) -> Self { Self([$({ let _ = stringify!($comp); v }),+]) }
            $(#[inline] pub fn $comp(&self) -> u32 { self.0[${index()}] })+
            #[inline] pub fn min(self, rhs: Self) -> Self { Self([$(self.0[$i].min(rhs.0[$i])),+]) }
            #[inline] pub fn max(self, rhs: Self) -> Self { Self([$(self.0[$i].max(rhs.0[$i])),+]) }
            #[inline] pub fn min_elem(self) -> u32 { let mut m = self.0[0]; $(m = m.min(self.0[$i]);)+ m }
            #[inline] pub fn max_elem(self) -> u32 { let mut m = self.0[0]; $(m = m.max(self.0[$i]);)+ m }
            #[inline] pub fn clamp(self, min: Self, max: Self) -> Self { Self([$(self.0[$i].clamp(min.0[$i], max.0[$i])),+]) }
            #[inline] pub fn to_float(self) -> $vecf { $vecf([$(self.0[$i] as f32),+]) }
            #[inline] pub fn to_int(self) -> $veci { $veci([$(self.0[$i] as i32),+]) }
        }

        impl From<$vecf> for $vec {
            #[inline] fn from(v: $vecf) -> Self { Self([$(v.0[$i].max(0.0) as u32),+]) }
        }
    };
}

// ============================================================================
// Define Vecs
// ============================================================================

define_vecf!(Vec2, Vec2u, Vec2i, 2, [0, 1], [x, y], impl_from_tuple2);
define_vecf!(
    Vec3,
    Vec3u,
    Vec3i,
    3,
    [0, 1, 2],
    [x, y, z],
    impl_from_tuple3
);
define_vecf!(
    Vec4,
    Vec4u,
    Vec4i,
    4,
    [0, 1, 2, 3],
    [x, y, z, w],
    impl_from_tuple4
);

define_veci!(Vec2i, Vec2, Vec2u, 2, [0, 1], [x, y], impl_from_tuple2);
define_veci!(
    Vec3i,
    Vec3,
    Vec3u,
    3,
    [0, 1, 2],
    [x, y, z],
    impl_from_tuple3
);
define_veci!(
    Vec4i,
    Vec4,
    Vec4u,
    4,
    [0, 1, 2, 3],
    [x, y, z, w],
    impl_from_tuple4
);

define_vecu!(Vec2u, Vec2, Vec2i, 2, [0, 1], [x, y], impl_from_tuple2);
define_vecu!(
    Vec3u,
    Vec3,
    Vec3i,
    3,
    [0, 1, 2],
    [x, y, z],
    impl_from_tuple3
);
define_vecu!(
    Vec4u,
    Vec4,
    Vec4i,
    4,
    [0, 1, 2, 3],
    [x, y, z, w],
    impl_from_tuple4
);

// ============================================================================
// Vec2
// ============================================================================

impl Vec2 {
    pub const X: Vec2 = Vec2::new(1.0, 0.0);
    pub const Y: Vec2 = Vec2::new(0.0, 1.0);

    /// CCW
    pub fn cross(self, rhs: Self) -> f32 {
        self.0[0] * rhs.0[1] - self.0[1] * rhs.0[0]
    }

    pub fn cross_cw(self, rhs: Self) -> f32 {
        self.0[1] * rhs.0[0] - self.0[0] * rhs.0[1]
    }
}

impl Vec2i {
    pub const X: Vec2i = Vec2i::new(1, 0);
    pub const Y: Vec2i = Vec2i::new(0, 1);
}

impl Vec2u {
    pub const X: Vec2u = Vec2u::new(1, 0);
    pub const Y: Vec2u = Vec2u::new(0, 1);
}

// ============================================================================
// Vec3
// ============================================================================

impl Vec3 {
    pub const X: Vec3 = Vec3::new(1.0, 0.0, 0.0);
    pub const Y: Vec3 = Vec3::new(0.0, 1.0, 0.0);
    pub const Z: Vec3 = Vec3::new(0.0, 0.0, 1.0);

    #[inline]
    pub fn cross(self, rhs: Self) -> Self {
        Self([
            self.0[1] * rhs.0[2] - self.0[2] * rhs.0[1],
            self.0[2] * rhs.0[0] - self.0[0] * rhs.0[2],
            self.0[0] * rhs.0[1] - self.0[1] * rhs.0[0],
        ])
    }
}

impl Vec3i {
    pub const X: Vec3i = Vec3i::new(1, 0, 0);
    pub const Y: Vec3i = Vec3i::new(0, 1, 0);
    pub const Z: Vec3i = Vec3i::new(0, 0, 1);

    #[inline]
    pub fn cross(self, rhs: Self) -> Self {
        Self([
            self.0[1] * rhs.0[2] - self.0[2] * rhs.0[1],
            self.0[2] * rhs.0[0] - self.0[0] * rhs.0[2],
            self.0[0] * rhs.0[1] - self.0[1] * rhs.0[0],
        ])
    }
}

impl Vec3u {
    pub const X: Vec3u = Vec3u::new(1, 0, 0);
    pub const Y: Vec3u = Vec3u::new(0, 1, 0);
    pub const Z: Vec3u = Vec3u::new(0, 0, 1);
}

// ============================================================================
// Vec4
// ============================================================================

impl Vec4 {
    pub const X: Vec4 = Vec4::new(1.0, 0.0, 0.0, 0.0);
    pub const Y: Vec4 = Vec4::new(0.0, 1.0, 0.0, 0.0);
    pub const Z: Vec4 = Vec4::new(0.0, 0.0, 1.0, 0.0);
    pub const W: Vec4 = Vec4::new(0.0, 0.0, 0.0, 1.0);
}

impl Vec4i {
    pub const X: Vec4i = Vec4i::new(1, 0, 0, 0);
    pub const Y: Vec4i = Vec4i::new(0, 1, 0, 0);
    pub const Z: Vec4i = Vec4i::new(0, 0, 1, 0);
    pub const W: Vec4i = Vec4i::new(0, 0, 0, 1);
}

impl Vec4u {
    pub const X: Vec4u = Vec4u::new(1, 0, 0, 0);
    pub const Y: Vec4u = Vec4u::new(0, 1, 0, 0);
    pub const Z: Vec4u = Vec4u::new(0, 0, 1, 0);
    pub const W: Vec4u = Vec4u::new(0, 0, 0, 1);
}
