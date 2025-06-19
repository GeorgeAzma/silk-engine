#![allow(unused)]
use super::rand::Rand;
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Neg, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};

pub trait ExtraFns:
    Sized + Copy + Add<Self, Output = Self> + Mul<Self, Output = Self> + Sub<Self, Output = Self>
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
    ($trait: ident, $method: ident, $wrap_method: ident, $ty: ident, $comp_ty: ty, $($comp: ident),+) => {
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
    ($trait: ident, $method: ident, $ty: ident, $comp_ty: ty, $($comp: ident),+) => {
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
    ($ty: ident, $tuple_ty: ty) => {
        impl From<$tuple_ty> for $ty {
            fn from(value: $tuple_ty) -> Self {
                unsafe { std::mem::transmute(value) }
            }
        }
    };
}

macro_rules! impl_from_tuple {
    ($ty: ident, $comp_ty: ty, $x: ident, $y: ident, $z: ident, $w: ident) => {
        impl_tuple_helper!($ty, (u32, u32, u32, u32));
        impl_tuple_helper!($ty, (i32, i32, i32, i32));
        impl_tuple_helper!($ty, (f32, f32, f32, f32));
    };
    ($ty: ident, $comp_ty: ty, $x: ident, $y: ident, $z: ident) => {
        impl_tuple_helper!($ty, (u32, u32, u32));
        impl_tuple_helper!($ty, (i32, i32, i32));
        impl_tuple_helper!($ty, (f32, f32, f32));
    };
    ($ty: ident, $comp_ty: ty, $x: ident, $y: ident) => {
        impl_tuple_helper!($ty, (u32, u32));
        impl_tuple_helper!($ty, (i32, i32));
        impl_tuple_helper!($ty, (f32, f32));
    };
}

macro_rules! impl_vecu {
    (+, $ty: ident, $comp_ty: ty, $first_comp: ident, $($comp: ident),+) => {
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

        impl_splat!($comp_ty, $first_comp, $($comp),*);
    };
}

macro_rules! impl_veci {
    (+, $ty: ident, $comp_ty: ty, $first_comp: ident, $($comp: ident),+) => {
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
    ($ty: ident, $vecu: ident, $first_comp: ident, $($comp: ident),+) => {
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
    ($ty: ident, $($comp: ident),+) => {
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
    ($ty: ident, $($comp: ident),+) => {
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
    ($ty: ident, $($comp: ident),+) => {
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
    ($ty: ident, u32, $($comp: ident),+) => {
        impl_op!(Add, add, wrapping_add, $ty, u32, $($comp),+);
        impl_op!(Sub, sub, wrapping_sub, $ty, u32, $($comp),+);
        impl_op!(Mul, mul, wrapping_mul, $ty, u32, $($comp),+);
        impl_op!(Div, div, wrapping_div, $ty, u32, $($comp),+);
        impl_op!(Rem, rem, rem, $ty, u32, $($comp),+);
        impl_op_assign!(AddAssign, add_assign, $ty, u32, $($comp),+);
        impl_op_assign!(SubAssign, sub_assign, $ty, u32, $($comp),+);
        impl_op_assign!(MulAssign, mul_assign, $ty, u32, $($comp),+);
        impl_op_assign!(DivAssign, div_assign, $ty, u32, $($comp),+);
        impl_op_assign!(RemAssign, rem_assign, $ty, u32, $($comp),+);
    };
    ($ty: ident, i32, $($comp: ident),+) => {
        impl_op!(Add, add, wrapping_add, $ty, i32, $($comp),+);
        impl_op!(Sub, sub, wrapping_sub, $ty, i32, $($comp),+);
        impl_op!(Mul, mul, wrapping_mul, $ty, i32, $($comp),+);
        impl_op!(Div, div, wrapping_div, $ty, i32, $($comp),+);
        impl_op!(Rem, rem, rem, $ty, i32, $($comp),+);
        impl_op_assign!(AddAssign, add_assign, $ty, i32, $($comp),+);
        impl_op_assign!(SubAssign, sub_assign, $ty, i32, $($comp),+);
        impl_op_assign!(MulAssign, mul_assign, $ty, i32, $($comp),+);
        impl_op_assign!(DivAssign, div_assign, $ty, i32, $($comp),+);
        impl_op_assign!(RemAssign, rem_assign, $ty, i32, $($comp),+);
    };
    ($ty: ident, $comp_ty: ty, $($comp: ident),+) => {
        impl_op!(Add, add, add, $ty, $comp_ty, $($comp),+);
        impl_op!(Sub, sub, sub, $ty, $comp_ty, $($comp),+);
        impl_op!(Mul, mul, mul, $ty, $comp_ty, $($comp),+);
        impl_op!(Div, div, div, $ty, $comp_ty, $($comp),+);
        impl_op!(Rem, rem, rem, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(AddAssign, add_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(SubAssign, sub_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(MulAssign, mul_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(DivAssign, div_assign, $ty, $comp_ty, $($comp),+);
        impl_op_assign!(RemAssign, rem_assign, $ty, $comp_ty, $($comp),+);

    };
}

macro_rules! impl_bitops {
    ($ty: ident, $comp_ty: ty, $($comp: ident),+) => {
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

macro_rules! vecn {
    (f32, $x: ident, $y: ident) => {
        Vec2
    };
    (f32, $x: ident, $y: ident, $z: ident) => {
        Vec3
    };
    (f32, $x: ident, $y: ident, $z: ident, $w: ident) => {
        Vec4
    };
    (i32, $x: ident, $y: ident) => {
        Vec2i
    };
    (i32, $x: ident, $y: ident, $z: ident) => {
        Vec3i
    };
    (i32, $x: ident, $y: ident, $z: ident, $w: ident) => {
        Vec4i
    };
    (u32, $x: ident, $y: ident) => {
        Vec2u
    };
    (u32, $x: ident, $y: ident, $z: ident) => {
        Vec3u
    };
    (u32, $x: ident, $y: ident, $z: ident, $w: ident) => {
        Vec4u
    };
}

macro_rules! swiz_fn {
    ($cty: ident, $name: ident, $($comp: ident),+) => {
        pub fn $name(self) -> vecn!($cty, $($comp),*) {
            <vecn!($cty, $($comp),*)>::new($(self.$comp),*)
        }
    };
}

macro_rules! impl_swiz {
    ($c: ident, x, y) => {
        swiz_fn!($c, xx, x, x);
        swiz_fn!($c, xy, x, y);
        swiz_fn!($c, yx, y, x);
        swiz_fn!($c, yy, y, y);
        swiz_fn!($c, xxx, x, x, x);
        swiz_fn!($c, xxy, x, x, y);
        swiz_fn!($c, xyx, x, y, x);
        swiz_fn!($c, xyy, x, y, y);
        swiz_fn!($c, yxx, y, x, x);
        swiz_fn!($c, yxy, y, x, y);
        swiz_fn!($c, yyx, y, y, x);
        swiz_fn!($c, yyy, y, y, y);
    };
    ($c: ident, x, y, z) => {
        swiz_fn!($c, xx, x, x);
        swiz_fn!($c, xy, x, y);
        swiz_fn!($c, xz, x, z);
        swiz_fn!($c, yx, y, x);
        swiz_fn!($c, yy, y, y);
        swiz_fn!($c, yz, y, z);
        swiz_fn!($c, zx, z, x);
        swiz_fn!($c, zy, z, y);
        swiz_fn!($c, zz, z, z);
        swiz_fn!($c, xxx, x, x, x);
        swiz_fn!($c, xxy, x, x, y);
        swiz_fn!($c, xxz, x, x, z);
        swiz_fn!($c, xyx, x, y, x);
        swiz_fn!($c, xyy, x, y, y);
        swiz_fn!($c, xyz, x, y, z);
        swiz_fn!($c, xzx, x, z, x);
        swiz_fn!($c, xzy, x, z, y);
        swiz_fn!($c, xzz, x, z, z);
        swiz_fn!($c, yxx, y, x, x);
        swiz_fn!($c, yxy, y, x, y);
        swiz_fn!($c, yxz, y, x, z);
        swiz_fn!($c, yyx, y, y, x);
        swiz_fn!($c, yyy, y, y, y);
        swiz_fn!($c, yyz, y, y, z);
        swiz_fn!($c, yzx, y, z, x);
        swiz_fn!($c, yzy, y, z, y);
        swiz_fn!($c, yzz, y, z, z);
        swiz_fn!($c, zxx, z, x, x);
        swiz_fn!($c, zxy, z, x, y);
        swiz_fn!($c, zxz, z, x, z);
        swiz_fn!($c, zyx, z, y, x);
        swiz_fn!($c, zyy, z, y, y);
        swiz_fn!($c, zyz, z, y, z);
        swiz_fn!($c, zzx, z, z, x);
        swiz_fn!($c, zzy, z, z, y);
        swiz_fn!($c, zzz, z, z, z);

        swiz_fn!($c, xxxx, x, x, x, x);
        swiz_fn!($c, xxxy, x, x, x, y);
        swiz_fn!($c, xxxz, x, x, x, z);
        swiz_fn!($c, xxyx, x, x, y, x);
        swiz_fn!($c, xxyy, x, x, y, y);
        swiz_fn!($c, xxyz, x, x, y, z);
        swiz_fn!($c, xxzx, x, x, z, x);
        swiz_fn!($c, xxzy, x, x, z, y);
        swiz_fn!($c, xxzz, x, x, z, z);
        swiz_fn!($c, xyxx, x, y, x, x);
        swiz_fn!($c, xyxy, x, y, x, y);
        swiz_fn!($c, xyxz, x, y, x, z);
        swiz_fn!($c, xyyx, x, y, y, x);
        swiz_fn!($c, xyyy, x, y, y, y);
        swiz_fn!($c, xyyz, x, y, y, z);
        swiz_fn!($c, xyzx, x, y, z, x);
        swiz_fn!($c, xyzy, x, y, z, y);
        swiz_fn!($c, xyzz, x, y, z, z);
        swiz_fn!($c, xzxx, x, z, x, x);
        swiz_fn!($c, xzxy, x, z, x, y);
        swiz_fn!($c, xzxz, x, z, x, z);
        swiz_fn!($c, xzyx, x, z, y, x);
        swiz_fn!($c, xzyy, x, z, y, y);
        swiz_fn!($c, xzyz, x, z, y, z);
        swiz_fn!($c, xzzx, x, z, z, x);
        swiz_fn!($c, xzzy, x, z, z, y);
        swiz_fn!($c, xzzz, x, z, z, z);

        swiz_fn!($c, yxxx, y, x, x, x);
        swiz_fn!($c, yxxy, y, x, x, y);
        swiz_fn!($c, yxxz, y, x, x, z);
        swiz_fn!($c, yxyx, y, x, y, x);
        swiz_fn!($c, yxyy, y, x, y, y);
        swiz_fn!($c, yxyz, y, x, y, z);
        swiz_fn!($c, yxzx, y, x, z, x);
        swiz_fn!($c, yxzy, y, x, z, y);
        swiz_fn!($c, yxzz, y, x, z, z);
        swiz_fn!($c, yyxx, y, y, x, x);
        swiz_fn!($c, yyxy, y, y, x, y);
        swiz_fn!($c, yyxz, y, y, x, z);
        swiz_fn!($c, yyyx, y, y, y, x);
        swiz_fn!($c, yyyy, y, y, y, y);
        swiz_fn!($c, yyyz, y, y, y, z);
        swiz_fn!($c, yyzx, y, y, z, x);
        swiz_fn!($c, yyzy, y, y, z, y);
        swiz_fn!($c, yyzz, y, y, z, z);
        swiz_fn!($c, yzxx, y, z, x, x);
        swiz_fn!($c, yzxy, y, z, x, y);
        swiz_fn!($c, yzxz, y, z, x, z);
        swiz_fn!($c, yzyx, y, z, y, x);
        swiz_fn!($c, yzyy, y, z, y, y);
        swiz_fn!($c, yzyz, y, z, y, z);
        swiz_fn!($c, yzzx, y, z, z, x);
        swiz_fn!($c, yzzy, y, z, z, y);
        swiz_fn!($c, yzzz, y, z, z, z);

        swiz_fn!($c, zxxx, z, x, x, x);
        swiz_fn!($c, zxxy, z, x, x, y);
        swiz_fn!($c, zxxz, z, x, x, z);
        swiz_fn!($c, zxyx, z, x, y, x);
        swiz_fn!($c, zxyy, z, x, y, y);
        swiz_fn!($c, zxyz, z, x, y, z);
        swiz_fn!($c, zxzx, z, x, z, x);
        swiz_fn!($c, zxzy, z, x, z, y);
        swiz_fn!($c, zxzz, z, x, z, z);
        swiz_fn!($c, zyxx, z, y, x, x);
        swiz_fn!($c, zyxy, z, y, x, y);
        swiz_fn!($c, zyxz, z, y, x, z);
        swiz_fn!($c, zyyx, z, y, y, x);
        swiz_fn!($c, zyyy, z, y, y, y);
        swiz_fn!($c, zyyz, z, y, y, z);
        swiz_fn!($c, zyzx, z, y, z, x);
        swiz_fn!($c, zyzy, z, y, z, y);
        swiz_fn!($c, zyzz, z, y, z, z);
        swiz_fn!($c, zzxx, z, z, x, x);
        swiz_fn!($c, zzxy, z, z, x, y);
        swiz_fn!($c, zzxz, z, z, x, z);
        swiz_fn!($c, zzyx, z, z, y, x);
        swiz_fn!($c, zzyy, z, z, y, y);
        swiz_fn!($c, zzyz, z, z, y, z);
        swiz_fn!($c, zzzx, z, z, z, x);
        swiz_fn!($c, zzzy, z, z, z, y);
        swiz_fn!($c, zzzz, z, z, z, z);
    };

    ($c: ident, x, y, z, w) => {
        swiz_fn!($c, xx, x, x);
        swiz_fn!($c, xy, x, y);
        swiz_fn!($c, xz, x, z);
        swiz_fn!($c, xw, x, w);
        swiz_fn!($c, yx, y, x);
        swiz_fn!($c, yy, y, y);
        swiz_fn!($c, yz, y, z);
        swiz_fn!($c, yw, y, w);
        swiz_fn!($c, zx, z, x);
        swiz_fn!($c, zy, z, y);
        swiz_fn!($c, zz, z, z);
        swiz_fn!($c, zw, z, w);
        swiz_fn!($c, wx, w, x);
        swiz_fn!($c, wy, w, y);
        swiz_fn!($c, wz, w, z);
        swiz_fn!($c, ww, w, w);
        swiz_fn!($c, xxx, x, x, x);
        swiz_fn!($c, xxy, x, x, y);
        swiz_fn!($c, xxz, x, x, z);
        swiz_fn!($c, xxw, x, x, w);
        swiz_fn!($c, xyx, x, y, x);
        swiz_fn!($c, xyy, x, y, y);
        swiz_fn!($c, xyz, x, y, z);
        swiz_fn!($c, xyw, x, y, w);
        swiz_fn!($c, xzx, x, z, x);
        swiz_fn!($c, xzy, x, z, y);
        swiz_fn!($c, xzz, x, z, z);
        swiz_fn!($c, xzw, x, z, w);
        swiz_fn!($c, xwx, x, w, x);
        swiz_fn!($c, xwy, x, w, y);
        swiz_fn!($c, xwz, x, w, z);
        swiz_fn!($c, xww, x, w, w);
        swiz_fn!($c, yxx, y, x, x);
        swiz_fn!($c, yxy, y, x, y);
        swiz_fn!($c, yxz, y, x, z);
        swiz_fn!($c, yxw, y, x, w);
        swiz_fn!($c, yyx, y, y, x);
        swiz_fn!($c, yyy, y, y, y);
        swiz_fn!($c, yyz, y, y, z);
        swiz_fn!($c, yyw, y, y, w);
        swiz_fn!($c, yzx, y, z, x);
        swiz_fn!($c, yzy, y, z, y);
        swiz_fn!($c, yzz, y, z, z);
        swiz_fn!($c, yzw, y, z, w);
        swiz_fn!($c, ywx, y, w, x);
        swiz_fn!($c, ywy, y, w, y);
        swiz_fn!($c, ywz, y, w, z);
        swiz_fn!($c, yww, y, w, w);
        swiz_fn!($c, zxx, z, x, x);
        swiz_fn!($c, zxy, z, x, y);
        swiz_fn!($c, zxz, z, x, z);
        swiz_fn!($c, zxw, z, x, w);
        swiz_fn!($c, zyx, z, y, x);
        swiz_fn!($c, zyy, z, y, y);
        swiz_fn!($c, zyz, z, y, z);
        swiz_fn!($c, zyw, z, y, w);
        swiz_fn!($c, zzx, z, z, x);
        swiz_fn!($c, zzy, z, z, y);
        swiz_fn!($c, zzz, z, z, z);
        swiz_fn!($c, zzw, z, z, w);
        swiz_fn!($c, zwx, z, w, x);
        swiz_fn!($c, zwy, z, w, y);
        swiz_fn!($c, zwz, z, w, z);
        swiz_fn!($c, zww, z, w, w);
        swiz_fn!($c, wxx, w, x, x);
        swiz_fn!($c, wxy, w, x, y);
        swiz_fn!($c, wxz, w, x, z);
        swiz_fn!($c, wxw, w, x, w);
        swiz_fn!($c, wyx, w, y, x);
        swiz_fn!($c, wyy, w, y, y);
        swiz_fn!($c, wyz, w, y, z);
        swiz_fn!($c, wyw, w, y, w);
        swiz_fn!($c, wzx, w, z, x);
        swiz_fn!($c, wzy, w, z, y);
        swiz_fn!($c, wzz, w, z, z);
        swiz_fn!($c, wzw, w, z, w);
        swiz_fn!($c, wwx, w, w, x);
        swiz_fn!($c, wwy, w, w, y);
        swiz_fn!($c, wwz, w, w, z);
        swiz_fn!($c, www, w, w, w);

        swiz_fn!($c, xxxx, x, x, x, x);
        swiz_fn!($c, xxxy, x, x, x, y);
        swiz_fn!($c, xxxz, x, x, x, z);
        swiz_fn!($c, xxxw, x, x, x, w);
        swiz_fn!($c, xxyx, x, x, y, x);
        swiz_fn!($c, xxyy, x, x, y, y);
        swiz_fn!($c, xxyz, x, x, y, z);
        swiz_fn!($c, xxyw, x, x, y, w);
        swiz_fn!($c, xxzx, x, x, z, x);
        swiz_fn!($c, xxzy, x, x, z, y);
        swiz_fn!($c, xxzz, x, x, z, z);
        swiz_fn!($c, xxzw, x, x, z, w);
        swiz_fn!($c, xxwx, x, x, w, x);
        swiz_fn!($c, xxwy, x, x, w, y);
        swiz_fn!($c, xxwz, x, x, w, z);
        swiz_fn!($c, xxww, x, x, w, w);
        swiz_fn!($c, xyxx, x, y, x, x);
        swiz_fn!($c, xyxy, x, y, x, y);
        swiz_fn!($c, xyxz, x, y, x, z);
        swiz_fn!($c, xyxw, x, y, x, w);
        swiz_fn!($c, xyyx, x, y, y, x);
        swiz_fn!($c, xyyy, x, y, y, y);
        swiz_fn!($c, xyyz, x, y, y, z);
        swiz_fn!($c, xyyw, x, y, y, w);
        swiz_fn!($c, xyzx, x, y, z, x);
        swiz_fn!($c, xyzy, x, y, z, y);
        swiz_fn!($c, xyzz, x, y, z, z);
        swiz_fn!($c, xyzw, x, y, z, w);
        swiz_fn!($c, xywx, x, y, w, x);
        swiz_fn!($c, xywy, x, y, w, y);
        swiz_fn!($c, xywz, x, y, w, z);
        swiz_fn!($c, xyww, x, y, w, w);
        swiz_fn!($c, xzxx, x, z, x, x);
        swiz_fn!($c, xzxy, x, z, x, y);
        swiz_fn!($c, xzxz, x, z, x, z);
        swiz_fn!($c, xzxw, x, z, x, w);
        swiz_fn!($c, xzyx, x, z, y, x);
        swiz_fn!($c, xzyy, x, z, y, y);
        swiz_fn!($c, xzyz, x, z, y, z);
        swiz_fn!($c, xzyw, x, z, y, w);
        swiz_fn!($c, xzzx, x, z, z, x);
        swiz_fn!($c, xzzy, x, z, z, y);
        swiz_fn!($c, xzzz, x, z, z, z);
        swiz_fn!($c, xzzw, x, z, z, w);
        swiz_fn!($c, xzwx, x, z, w, x);
        swiz_fn!($c, xzwy, x, z, w, y);
        swiz_fn!($c, xzwz, x, z, w, z);
        swiz_fn!($c, xzww, x, z, w, w);
        swiz_fn!($c, xwxx, x, w, x, x);
        swiz_fn!($c, xwxy, x, w, x, y);
        swiz_fn!($c, xwxz, x, w, x, z);
        swiz_fn!($c, xwxw, x, w, x, w);
        swiz_fn!($c, xwyx, x, w, y, x);
        swiz_fn!($c, xwyy, x, w, y, y);
        swiz_fn!($c, xwyz, x, w, y, z);
        swiz_fn!($c, xwyw, x, w, y, w);
        swiz_fn!($c, xwzx, x, w, z, x);
        swiz_fn!($c, xwzy, x, w, z, y);
        swiz_fn!($c, xwzz, x, w, z, z);
        swiz_fn!($c, xwzw, x, w, z, w);
        swiz_fn!($c, xwwx, x, w, w, x);
        swiz_fn!($c, xwwy, x, w, w, y);
        swiz_fn!($c, xwwz, x, w, w, z);
        swiz_fn!($c, xwww, x, w, w, w);

        swiz_fn!($c, yxxx, y, x, x, x);
        swiz_fn!($c, yxxy, y, x, x, y);
        swiz_fn!($c, yxxz, y, x, x, z);
        swiz_fn!($c, yxxw, y, x, x, w);
        swiz_fn!($c, yxyx, y, x, y, x);
        swiz_fn!($c, yxyy, y, x, y, y);
        swiz_fn!($c, yxyz, y, x, y, z);
        swiz_fn!($c, yxyw, y, x, y, w);
        swiz_fn!($c, yxzx, y, x, z, x);
        swiz_fn!($c, yxzy, y, x, z, y);
        swiz_fn!($c, yxzz, y, x, z, z);
        swiz_fn!($c, yxzw, y, x, z, w);
        swiz_fn!($c, yxwx, y, x, w, x);
        swiz_fn!($c, yxwy, y, x, w, y);
        swiz_fn!($c, yxwz, y, x, w, z);
        swiz_fn!($c, yxww, y, x, w, w);
        swiz_fn!($c, yyxx, y, y, x, x);
        swiz_fn!($c, yyxy, y, y, x, y);
        swiz_fn!($c, yyxz, y, y, x, z);
        swiz_fn!($c, yyxw, y, y, x, w);
        swiz_fn!($c, yyyx, y, y, y, x);
        swiz_fn!($c, yyyy, y, y, y, y);
        swiz_fn!($c, yyyz, y, y, y, z);
        swiz_fn!($c, yyyw, y, y, y, w);
        swiz_fn!($c, yyzx, y, y, z, x);
        swiz_fn!($c, yyzy, y, y, z, y);
        swiz_fn!($c, yyzz, y, y, z, z);
        swiz_fn!($c, yyzw, y, y, z, w);
        swiz_fn!($c, yywx, y, y, w, x);
        swiz_fn!($c, yywy, y, y, w, y);
        swiz_fn!($c, yywz, y, y, w, z);
        swiz_fn!($c, yyww, y, y, w, w);
        swiz_fn!($c, yzxx, y, z, x, x);
        swiz_fn!($c, yzxy, y, z, x, y);
        swiz_fn!($c, yzxz, y, z, x, z);
        swiz_fn!($c, yzxw, y, z, x, w);
        swiz_fn!($c, yzyx, y, z, y, x);
        swiz_fn!($c, yzyy, y, z, y, y);
        swiz_fn!($c, yzyz, y, z, y, z);
        swiz_fn!($c, yzyw, y, z, y, w);
        swiz_fn!($c, yzzx, y, z, z, x);
        swiz_fn!($c, yzzy, y, z, z, y);
        swiz_fn!($c, yzzz, y, z, z, z);
        swiz_fn!($c, yzzw, y, z, z, w);
        swiz_fn!($c, yzwx, y, z, w, x);
        swiz_fn!($c, yzwy, y, z, w, y);
        swiz_fn!($c, yzwz, y, z, w, z);
        swiz_fn!($c, yzww, y, z, w, w);
        swiz_fn!($c, ywxx, y, w, x, x);
        swiz_fn!($c, ywxy, y, w, x, y);
        swiz_fn!($c, ywxz, y, w, x, z);
        swiz_fn!($c, ywxw, y, w, x, w);
        swiz_fn!($c, ywyx, y, w, y, x);
        swiz_fn!($c, ywyy, y, w, y, y);
        swiz_fn!($c, ywyz, y, w, y, z);
        swiz_fn!($c, ywyw, y, w, y, w);
        swiz_fn!($c, ywzx, y, w, z, x);
        swiz_fn!($c, ywzy, y, w, z, y);
        swiz_fn!($c, ywzz, y, w, z, z);
        swiz_fn!($c, ywzw, y, w, z, w);
        swiz_fn!($c, ywwx, y, w, w, x);
        swiz_fn!($c, ywwy, y, w, w, y);
        swiz_fn!($c, ywwz, y, w, w, z);
        swiz_fn!($c, ywww, y, w, w, w);

        swiz_fn!($c, zxxx, w, x, x, x);
        swiz_fn!($c, zxxy, w, x, x, y);
        swiz_fn!($c, zxxz, w, x, x, z);
        swiz_fn!($c, zxxw, w, x, x, w);
        swiz_fn!($c, zxyx, w, x, y, x);
        swiz_fn!($c, zxyy, w, x, y, y);
        swiz_fn!($c, zxyz, w, x, y, z);
        swiz_fn!($c, zxyw, w, x, y, w);
        swiz_fn!($c, zxzx, w, x, z, x);
        swiz_fn!($c, zxzy, w, x, z, y);
        swiz_fn!($c, zxzz, w, x, z, z);
        swiz_fn!($c, zxzw, w, x, z, w);
        swiz_fn!($c, zxwx, w, x, w, x);
        swiz_fn!($c, zxwy, w, x, w, y);
        swiz_fn!($c, zxwz, w, x, w, z);
        swiz_fn!($c, zxww, w, x, w, w);
        swiz_fn!($c, zyxx, w, y, x, x);
        swiz_fn!($c, zyxy, w, y, x, y);
        swiz_fn!($c, zyxz, w, y, x, z);
        swiz_fn!($c, zyxw, w, y, x, w);
        swiz_fn!($c, zyyx, w, y, y, x);
        swiz_fn!($c, zyyy, w, y, y, y);
        swiz_fn!($c, zyyz, w, y, y, z);
        swiz_fn!($c, zyyw, w, y, y, w);
        swiz_fn!($c, zyzx, w, y, z, x);
        swiz_fn!($c, zyzy, w, y, z, y);
        swiz_fn!($c, zyzz, w, y, z, z);
        swiz_fn!($c, zyzw, w, y, z, w);
        swiz_fn!($c, zywx, w, y, w, x);
        swiz_fn!($c, zywy, w, y, w, y);
        swiz_fn!($c, zywz, w, y, w, z);
        swiz_fn!($c, zyww, w, y, w, w);
        swiz_fn!($c, zzxx, w, z, x, x);
        swiz_fn!($c, zzxy, w, z, x, y);
        swiz_fn!($c, zzxz, w, z, x, z);
        swiz_fn!($c, zzxw, w, z, x, w);
        swiz_fn!($c, zzyx, w, z, y, x);
        swiz_fn!($c, zzyy, w, z, y, y);
        swiz_fn!($c, zzyz, w, z, y, z);
        swiz_fn!($c, zzyw, w, z, y, w);
        swiz_fn!($c, zzzx, w, z, z, x);
        swiz_fn!($c, zzzy, w, z, z, y);
        swiz_fn!($c, zzzz, w, z, z, z);
        swiz_fn!($c, zzzw, w, z, z, w);
        swiz_fn!($c, zzwx, w, z, w, x);
        swiz_fn!($c, zzwy, w, z, w, y);
        swiz_fn!($c, zzwz, w, z, w, z);
        swiz_fn!($c, zzww, w, z, w, w);
        swiz_fn!($c, zwxx, w, w, x, x);
        swiz_fn!($c, zwxy, w, w, x, y);
        swiz_fn!($c, zwxz, w, w, x, z);
        swiz_fn!($c, zwxw, w, w, x, w);
        swiz_fn!($c, zwyx, w, w, y, x);
        swiz_fn!($c, zwyy, w, w, y, y);
        swiz_fn!($c, zwyz, w, w, y, z);
        swiz_fn!($c, zwyw, w, w, y, w);
        swiz_fn!($c, zwzx, w, w, z, x);
        swiz_fn!($c, zwzy, w, w, z, y);
        swiz_fn!($c, zwzz, w, w, z, z);
        swiz_fn!($c, zwzw, w, w, z, w);
        swiz_fn!($c, zwwx, w, w, w, x);
        swiz_fn!($c, zwwy, w, w, w, y);
        swiz_fn!($c, zwwz, w, w, w, z);
        swiz_fn!($c, zwww, w, w, w, w);

        swiz_fn!($c, wxxx, w, x, x, x);
        swiz_fn!($c, wxxy, w, x, x, y);
        swiz_fn!($c, wxxz, w, x, x, z);
        swiz_fn!($c, wxxw, w, x, x, w);
        swiz_fn!($c, wxyx, w, x, y, x);
        swiz_fn!($c, wxyy, w, x, y, y);
        swiz_fn!($c, wxyz, w, x, y, z);
        swiz_fn!($c, wxyw, w, x, y, w);
        swiz_fn!($c, wxzx, w, x, z, x);
        swiz_fn!($c, wxzy, w, x, z, y);
        swiz_fn!($c, wxzz, w, x, z, z);
        swiz_fn!($c, wxzw, w, x, z, w);
        swiz_fn!($c, wxwx, w, x, w, x);
        swiz_fn!($c, wxwy, w, x, w, y);
        swiz_fn!($c, wxwz, w, x, w, z);
        swiz_fn!($c, wxww, w, x, w, w);
        swiz_fn!($c, wyxx, w, y, x, x);
        swiz_fn!($c, wyxy, w, y, x, y);
        swiz_fn!($c, wyxz, w, y, x, z);
        swiz_fn!($c, wyxw, w, y, x, w);
        swiz_fn!($c, wyyx, w, y, y, x);
        swiz_fn!($c, wyyy, w, y, y, y);
        swiz_fn!($c, wyyz, w, y, y, z);
        swiz_fn!($c, wyyw, w, y, y, w);
        swiz_fn!($c, wyzx, w, y, z, x);
        swiz_fn!($c, wyzy, w, y, z, y);
        swiz_fn!($c, wyzz, w, y, z, z);
        swiz_fn!($c, wyzw, w, y, z, w);
        swiz_fn!($c, wywx, w, y, w, x);
        swiz_fn!($c, wywy, w, y, w, y);
        swiz_fn!($c, wywz, w, y, w, z);
        swiz_fn!($c, wyww, w, y, w, w);
        swiz_fn!($c, wzxx, w, z, x, x);
        swiz_fn!($c, wzxy, w, z, x, y);
        swiz_fn!($c, wzxz, w, z, x, z);
        swiz_fn!($c, wzxw, w, z, x, w);
        swiz_fn!($c, wzyx, w, z, y, x);
        swiz_fn!($c, wzyy, w, z, y, y);
        swiz_fn!($c, wzyz, w, z, y, z);
        swiz_fn!($c, wzyw, w, z, y, w);
        swiz_fn!($c, wzzx, w, z, z, x);
        swiz_fn!($c, wzzy, w, z, z, y);
        swiz_fn!($c, wzzz, w, z, z, z);
        swiz_fn!($c, wzzw, w, z, z, w);
        swiz_fn!($c, wzwx, w, z, w, x);
        swiz_fn!($c, wzwy, w, z, w, y);
        swiz_fn!($c, wzwz, w, z, w, z);
        swiz_fn!($c, wzww, w, z, w, w);
        swiz_fn!($c, wwxx, w, w, x, x);
        swiz_fn!($c, wwxy, w, w, x, y);
        swiz_fn!($c, wwxz, w, w, x, z);
        swiz_fn!($c, wwxw, w, w, x, w);
        swiz_fn!($c, wwyx, w, w, y, x);
        swiz_fn!($c, wwyy, w, w, y, y);
        swiz_fn!($c, wwyz, w, w, y, z);
        swiz_fn!($c, wwyw, w, w, y, w);
        swiz_fn!($c, wwzx, w, w, z, x);
        swiz_fn!($c, wwzy, w, w, z, y);
        swiz_fn!($c, wwzz, w, w, z, z);
        swiz_fn!($c, wwzw, w, w, z, w);
        swiz_fn!($c, wwwx, w, w, w, x);
        swiz_fn!($c, wwwy, w, w, w, y);
        swiz_fn!($c, wwwz, w, w, w, z);
        swiz_fn!($c, wwww, w, w, w, w);
    };
}

macro_rules! def_const {
    ($name: ident, $cty: ty, $($comp: expr),+) => {
        pub const $name: Self = Self::new($($comp as $cty),*);
    };
}

macro_rules! def_vec_neg_consts {
    ($ty: ident, $cty: ty, $x: ident, $y: ident, $z: ident, $w: ident) => {
        def_const!(NEG_ONE, $cty, -1, -1, -1, -1);
        def_const!(NEG_X, $cty, -1, 0, 0, 0);
        def_const!(NEG_Y, $cty, 0, -1, 0, 0);
        def_const!(NEG_Z, $cty, 0, 0, -1, 0);
        def_const!(NEG_W, $cty, 0, 0, 0, -1);
        def_const!(NEG_XY, $cty, -1, -1, 0, 0);
        def_const!(NEG_YZ, $cty, 0, -1, -1, 0);
        def_const!(NEG_ZW, $cty, 0, 0, -1, -1);
        def_const!(NEG_XZ, $cty, -1, 0, -1, 0);
        def_const!(NEG_YW, $cty, 0, -1, 0, -1);
        def_const!(NEG_XYZ, $cty, -1, -1, -1, 0);
        def_const!(NEG_YZW, $cty, 0, -1, -1, -1);
        def_const!(NEG_XYZW, $cty, -1, -1, -1, -1);
    };
    ($ty: ident, $cty: ty, $x: ident, $y: ident, $z: ident) => {
        def_const!(NEG_ONE, $cty, -1, -1, -1);
        def_const!(NEG_X, $cty, -1, 0, 0);
        def_const!(NEG_Y, $cty, 0, -1, 0);
        def_const!(NEG_Z, $cty, 0, 0, -1);
        def_const!(NEG_XY, $cty, -1, -1, 0);
        def_const!(NEG_YZ, $cty, 0, -1, -1);
        def_const!(NEG_XZ, $cty, -1, 0, -1);
        def_const!(NEG_XYZ, $cty, -1, -1, -1);
    };
    ($ty: ident, $cty: ty, $x: ident, $y: ident) => {
        def_const!(NEG_ONE, $cty, -1, -1);
        def_const!(NEG_X, $cty, -1, 0);
        def_const!(NEG_Y, $cty, 0, -1);
        def_const!(NEG_XY, $cty, -1, -1);
    };
}

macro_rules! def_vec_consts {
    (-, $ty: ident, i32, $($comp: ident),+) => {
        def_vec_neg_consts!($ty, i32, $($comp),*);
        def_vec_consts!(+, $ty, i32, $($comp),*);
    };
    (-, $ty: ident, f32, $($comp: ident),+) => {
        def_vec_neg_consts!($ty, f32, $($comp),*);
        def_vec_consts!(+, $ty, f32, $($comp),*);
    };
    (-, $ty: ident, $cty: ty, $($comp: ident),+) => {
        def_vec_consts!(+, $ty, $cty, $($comp),*);
    };
    (+, $ty: ident, $cty: ty, $x: ident, $y: ident, $z: ident, $w: ident) => {
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
        def_const!(XY, $cty, 1, 1, 0, 0);
        def_const!(YZ, $cty, 0, 1, 1, 0);
        def_const!(ZW, $cty, 0, 0, 1, 1);
        def_const!(XZ, $cty, 1, 0, 1, 0);
        def_const!(YW, $cty, 0, 1, 0, 1);
        def_const!(XYZ, $cty, 1, 1, 1, 0);
        def_const!(YZW, $cty, 0, 1, 1, 1);
        def_const!(XYZW, $cty, 1, 1, 1, 1);
    };
    (+, $ty: ident, $cty: ty, $x: ident, $y: ident, $z: ident) => {
        def_const!(ZERO, $cty, 0, 0, 0);
        def_const!(ONE, $cty, 1, 1, 1);
        def_const!(MIN, $cty, <$cty>::MIN, <$cty>::MIN, <$cty>::MIN);
        def_const!(MAX, $cty, <$cty>::MAX, <$cty>::MAX, <$cty>::MAX);
        def_const!(X, $cty, 1, 0, 0);
        def_const!(Y, $cty, 0, 1, 0);
        def_const!(Z, $cty, 0, 0, 1);
        def_const!(XY, $cty, 1, 1, 0);
        def_const!(YZ, $cty, 0, 1, 1);
        def_const!(XZ, $cty, 1, 0, 1);
        def_const!(XYZ, $cty, 1, 1, 1);
    };
    (+, $ty: ident, $cty: ty, $x: ident, $y: ident) => {
        def_const!(ZERO, $cty, 0, 0);
        def_const!(ONE, $cty, 1, 1);
        def_const!(MIN, $cty, <$cty>::MIN, <$cty>::MIN);
        def_const!(MAX, $cty, <$cty>::MAX, <$cty>::MAX);
        def_const!(X, $cty, 1, 0);
        def_const!(Y, $cty, 0, 1);
        def_const!(XY, $cty, 1, 1);
    };
}

macro_rules! def_vec {
    ($ty: ident, $comp_ty: ident, $($comp: ident),+) => {
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

            impl_swiz!($comp_ty, $($comp),*);

            pub const fn new($($comp: $comp_ty),+) -> Self {
                Self { $($comp),* }
            }
        }
    };
}

macro_rules! def_vecu {
    ($ty: ident, $($comps: ident),+) => {
        def_vec!($ty, u32, $($comps),*);
        impl_bitops!($ty, u32, $($comps),*);
        impl_rand!($ty, $($comps),*);
        impl_vecu!(+, $ty, u32, $($comps),*);
    };
}

macro_rules! def_veci {
    ($ty: ident, $($comps: ident),+) => {
        def_vec!($ty, i32, $($comps),*);
        impl_bitops!($ty, i32, $($comps),*);
        impl_rand!($ty, $($comps),*);
        impl_veci!(+, $ty, i32, $($comps),*);
        impl_neg!($ty, $($comps),*);
    };
}

macro_rules! def_vecf {
    ($ty: ident, $vecu: ident, $($comps: ident),+) => {
        def_vec!($ty, f32, $($comps),*);
        impl_vecf_extra!($ty, $($comps),*);
        impl_vecf!($ty, $vecu, $($comps),*);
    };
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
    fn rem_euclid(self, div: Self) -> Self;
}

def_vecu!(Vec2u, x, y);
def_vecu!(Vec3u, x, y, z);
def_vecu!(Vec4u, x, y, z, w);

def_vecf!(Vec2, Vec2u, x, y);
def_vecf!(Vec3, Vec3u, x, y, z);
def_vecf!(Vec4, Vec4u, x, y, z, w);

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
