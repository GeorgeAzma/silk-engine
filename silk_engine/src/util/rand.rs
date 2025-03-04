use super::{ExtraFns, Vec2, Vec3, Vectorf};
use crate::swiz;
use std::ops::Add;

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

pub trait Noise:
    Sized + ExtraFns + Copy + From<f32> + std::ops::MulAssign + std::ops::MulAssign<f32>
{
    fn hash(self) -> f32;

    fn noise(self) -> f32;
    fn voronoise(self, #[allow(unused)] smooth: f32) -> f32 {
        panic!("type did not implement voronoise");
    }
    fn worley(self) -> f32 {
        panic!("type did not implement worley noise");
    }

    fn noise_tile(self, #[allow(unused)] scale: Self) -> f32 {
        panic!("type did not implement tiled noise");
    }
    fn voronoise_tile(self, #[allow(unused)] smooth: f32, #[allow(unused)] scale: Self) -> f32 {
        panic!("type did not implement tiled voronoise");
    }
    fn worley_tile(self, #[allow(unused)] scale: Self) -> f32 {
        panic!("type did not implement tiled worley noise");
    }

    fn fbm(self, oct: u32) -> f32 {
        self.fbm_fn(
            |p, a| {
                let n = p.noise();
                *a *= 0.5;
                *p *= 2.0;
                n
            },
            oct,
        )
    }
    fn fbm_fn(self, mut f: impl FnMut(&mut Self, &mut f32) -> f32, oct: u32) -> f32 {
        let mut s = 0.0;
        let mut m = 0.0;
        let mut a = 1.0;
        let mut p = self;
        for _ in 0..oct {
            m += a;
            s += a * f(&mut p, &mut a);
        }
        s / m
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
        res
    }

    fn noise_tile(self, scale: Self) -> f32 {
        let ip = self.floor();
        let u = self.fract().smooth();
        let hash = |x: f32, y: f32| Self::new(x, y).rem_euclid(scale).hash();
        let res = hash(ip.x, ip.y).lerp(hash(ip.x + 1.0, ip.y), u.x).lerp(
            hash(ip.x, ip.y + 1.0).lerp(hash(ip.x + 1.0, ip.y + 1.0), u.x),
            u.y,
        );
        res
    }

    fn voronoise(self, smooth: f32) -> f32 {
        let hash32 = |p: Vec2| {
            let mut p3 = (swiz!(Vec3, p. x y x) / Vec3::new(0.1031, 0.1030, 0.0973)).fract();
            p3 += p3.dot(swiz!(Vec3, p3. y x z) + 33.33);
            return ((swiz!(Vec3, p3. x x y) + swiz!(Vec3, p3. y z z)) * swiz!(Vec3, p3. z y x))
                .fract();
        };
        let smooth = 1.0 / smooth;
        let p = self.floor();
        let f = self - p;
        let mut va = 0.0;
        let mut wt = 0.0;
        for x in -1..=1 {
            for y in -1..=1 {
                let c = Vec2::new(x as f32, y as f32);
                let o = hash32(p + c);
                let d = (c - f + Vec2::new(o.x, o.y)).len();
                let ww = d.sstep(1.414, 0.0).powf(smooth);
                va += o.z * ww;
                wt += ww;
            }
        }
        return va / wt;
    }

    fn worley(self) -> f32 {
        let id = self.floor();
        let p = self - id;
        let mut w = 1e9f32;
        for x in -1..=1 {
            for y in -1..=1 {
                let c = Self::new(x as f32, y as f32);
                let c = p - c - (id + c).hash();
                w = w.min(c.len2());
            }
        }
        return 1.0 - w.sqrt();
    }

    fn worley_tile(self, scale: Self) -> f32 {
        let id = self.floor();
        let p = self - id;
        let mut w = 1e9f32;
        for x in -1..=1 {
            for y in -1..=1 {
                let c = Self::new(x as f32, y as f32);
                let c = p - c - (id + c).rem_euclid(scale).hash();
                w = w.min(c.len2());
            }
        }
        return 1.0 - w.sqrt();
    }
}

impl Noise for Vec3 {
    fn hash(self) -> f32 {
        let ux = (self.x * 141421356.0).to_bits();
        let uy = (self.y * 2718281828.0).to_bits();
        let uz = (self.z * 1618033988.0).to_bits();
        ((ux ^ uy ^ uz) * 3141592653u32) as f32 / u32::MAX as f32
    }

    fn noise(self) -> f32 {
        let ip = self.floor();
        let u = self.fract().smooth();

        let res = ip
            .hash()
            .lerp(Vec3::new(ip.x + 1.0, ip.y, ip.z).hash(), u.x)
            .lerp(
                Vec3::new(ip.x, ip.y + 1.0, ip.z)
                    .hash()
                    .lerp(Vec3::new(ip.x + 1.0, ip.y + 1.0, ip.z).hash(), u.x),
                u.y,
            )
            .lerp(
                Vec3::new(ip.x, ip.y, ip.z + 1.0)
                    .hash()
                    .lerp(Vec3::new(ip.x + 1.0, ip.y, ip.z + 1.0).hash(), u.x)
                    .lerp(
                        Vec3::new(ip.x, ip.y + 1.0, ip.z + 1.0)
                            .hash()
                            .lerp(Vec3::new(ip.x + 1.0, ip.y + 1.0, ip.z + 1.0).hash(), u.x),
                        u.y,
                    ),
                u.z,
            );

        res
    }

    fn noise_tile(self, scale: Self) -> f32 {
        let ip = self.floor();
        let u = self.fract().smooth();
        let hash = |x: f32, y: f32, z: f32| Vec3::new(x, y, z).rem_euclid(scale).hash();
        let res = hash(ip.x, ip.y, ip.z)
            .lerp(hash(ip.x + 1.0, ip.y, ip.z), u.x)
            .lerp(
                hash(ip.x, ip.y + 1.0, ip.z).lerp(hash(ip.x + 1.0, ip.y + 1.0, ip.z), u.x),
                u.y,
            )
            .lerp(
                hash(ip.x, ip.y, ip.z + 1.0)
                    .lerp(hash(ip.x + 1.0, ip.y, ip.z + 1.0), u.x)
                    .lerp(
                        hash(ip.x, ip.y + 1.0, ip.z + 1.0)
                            .lerp(hash(ip.x + 1.0, ip.y + 1.0, ip.z + 1.0), u.x),
                        u.y,
                    ),
                u.z,
            );

        res
    }

    fn worley(self) -> f32 {
        let id = self.floor();
        let p = self - id;
        let mut w = 1e9f32;
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let c = Vec3::new(x as f32, y as f32, z as f32);
                    let c = p - c - (id + c).hash();
                    w = w.min(c.len2());
                }
            }
        }
        return 1.0 - w.sqrt();
    }

    fn worley_tile(self, scale: Self) -> f32 {
        let id = self.floor();
        let p = self - id;
        let mut w = 1e9f32;
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let c = Self::new(x as f32, y as f32, z as f32);
                    let c = p - c - (id + c).rem_euclid(scale).hash();
                    w = w.min(c.len2());
                }
            }
        }
        return 1.0 - w.sqrt();
    }
}
