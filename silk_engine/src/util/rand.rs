use std::ops::Add;

use super::ExtraFns;

pub trait Rand: Sized {
    fn rand(self) -> Self;
    fn randn(self) -> Self {
        panic!("int types do not implement randn")
    }
    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self;
}

impl Rand for u8 {
    fn rand(self) -> Self {
        let mut x = self.wrapping_add(0x5D);
        x = (x ^ (x >> 4)).wrapping_mul(0x1D);
        x = (x ^ (x >> 3)).wrapping_mul(0x3B);
        x ^= x >> 3;
        x
    }

    fn randn(self) -> Self {
        (f32::from_bits(self as u32).randn() * u8::MAX as f32) as Self
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        f32::from_bits(self as u32).randn_range(min as f32, max as f32, stdev) as Self
    }
}

impl Rand for u16 {
    fn rand(self) -> Self {
        let mut x = self.wrapping_add(0x9E3D);
        x = (x ^ (x >> 8)).wrapping_mul(0x2F1D);
        x = (x ^ (x >> 7)).wrapping_mul(0x623B);
        x ^= x >> 7;
        x
    }

    fn randn(self) -> Self {
        (f32::from_bits(self as u32).randn() * u16::MAX as f32) as Self
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        f32::from_bits(self as u32).randn_range(min as f32, max as f32, stdev) as Self
    }
}

impl Rand for u32 {
    fn rand(self) -> Self {
        let mut x = self.wrapping_add(0x9E3779B9);
        x = (x ^ (x >> 16)).wrapping_mul(0x21F0AAAD);
        x = (x ^ (x >> 15)).wrapping_mul(0x735A2D97);
        x ^= x >> 15;
        x
    }

    fn randn(self) -> Self {
        (f32::from_bits(self).randn() * u32::MAX as f32) as Self
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        f32::from_bits(self).randn_range(min as f32, max as f32, stdev) as Self
    }
}

impl Rand for u64 {
    fn rand(self) -> Self {
        let mut x = self.wrapping_add(0x9E3779B97F4A7C15);
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
        x ^ (x >> 31)
    }

    fn randn(self) -> Self {
        (f64::from_bits(self).randn() * u64::MAX as f64) as Self
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        f64::from_bits(self).randn_range(min as f64, max as f64, stdev) as Self
    }
}

impl Rand for usize {
    fn rand(self) -> Self {
        let mut x = self.wrapping_add(0x9E3779B97F4A7C15);
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
        x ^ (x >> 31)
    }

    fn randn(self) -> Self {
        (f64::from_bits(self as u64).randn() * u64::MAX as f64) as Self
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        f64::from_bits(self as u64).randn_range(min as f64, max as f64, stdev) as Self
    }
}

impl Rand for i8 {
    fn rand(self) -> Self {
        (self as u8).rand() as Self
    }

    fn randn(self) -> Self {
        (f32::from_bits(self as u32).randn() * i8::MAX as f32) as Self
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        f32::from_bits(self as u32).randn_range(min as f32, max as f32, stdev) as Self
    }
}

impl Rand for i16 {
    fn rand(self) -> Self {
        (self as u16).rand() as Self
    }

    fn randn(self) -> Self {
        (f32::from_bits(self as u32).randn() * i16::MAX as f32) as Self
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        f32::from_bits(self as u32).randn_range(min as f32, max as f32, stdev) as Self
    }
}

impl Rand for i32 {
    fn rand(self) -> Self {
        (self as u32).rand() as Self
    }

    fn randn(self) -> Self {
        (f32::from_bits(self as u32).randn() * i32::MAX as f32) as Self
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        f32::from_bits(self as u32).randn_range(min as f32, max as f32, stdev) as Self
    }
}

impl Rand for i64 {
    fn rand(self) -> Self {
        (self as u64).rand() as Self
    }

    fn randn(self) -> Self {
        (f64::from_bits(self as u64).randn() * i64::MAX as f64) as Self
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        f64::from_bits(self as u64).randn_range(min as f64, max as f64, stdev) as Self
    }
}

impl Rand for isize {
    fn rand(self) -> Self {
        (self as usize).rand() as Self
    }

    fn randn(self) -> Self {
        (f64::from_bits(self as u64).randn() * u64::MAX as f64) as Self
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        f64::from_bits(self as u64).randn_range(min as f64, max as f64, stdev) as Self
    }
}

impl Rand for f32 {
    fn rand(mut self) -> Self {
        self *= 3141592653.0;
        let u32b = self.to_bits();
        (u32b.wrapping_mul(u32b).wrapping_mul(3141592653)) as f32 / (u32::MAX as f32)
    }

    fn randn(self) -> Self {
        let a = self.rand();
        let b = (self.add(271828182845904523.536028747135266249)).rand();
        let r = (-2.0 * a.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * b;
        r * theta.cos()
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        let mean = (min + max) * 0.5;
        let stdev = (max - min) / stdev;
        let z = self.randn();
        (mean + stdev * z).clamp(min, max)
    }
}

impl Rand for f64 {
    fn rand(mut self) -> Self {
        self *= 3141592653589793238.0;
        let u64b = self.to_bits();
        (u64b.wrapping_mul(u64b).wrapping_mul(3141592653589793238)) as f64 / (u64::MAX as f64)
    }

    fn randn(self) -> Self {
        let a = self.rand();
        let b = (self.add(271828182845904523536028747135.26624977572470936999595749669676)).rand();
        let r = (-2.0 * a.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * b;
        r * theta.cos()
    }

    fn randn_range(self, min: Self, max: Self, stdev: f32) -> Self {
        let mean = (min + max) * 0.5;
        let stdev = (max - min) / stdev as Self;
        let z = self.randn();
        mean + stdev * z
    }
}

pub trait Noise: Sized + ExtraFns + Copy + From<f32> + std::ops::MulAssign {
    fn hash(self) -> f32;
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
        s / m
    }
    fn voronoise(self, #[allow(unused)] smooth: f32) -> f32 {
        panic!("type did not implement voronoise");
    }
}

impl Noise for f32 {
    fn hash(self) -> f32 {
        self.rand()
    }

    fn noise(self) -> f32 {
        let fl = self.abs().floor();
        let fr = self.abs().fract();
        fl.rand().lerp((fl + 1.0).rand(), fr.smooth())
    }
}
