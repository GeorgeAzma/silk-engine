use super::vec::{ExtraFns, Vec2, Vec2u, Vec3, Vec3u, Vec4, Vectorf};

pub trait Noise:
    Sized + ExtraFns + Copy + From<f32> + std::ops::MulAssign + std::ops::MulAssign<f32>
{
    fn hash(self) -> f32 {
        panic!("type did not implement hash");
    }
    fn hash2(self) -> Vec2 {
        panic!("type did not implement hash2");
    }
    fn hash3(self) -> Vec3 {
        panic!("type did not implement hash3");
    }
    fn hash4(self) -> Vec4 {
        panic!("type did not implement hash4");
    }

    fn value(self) -> f32 {
        panic!("type did not implement value noise");
    }
    fn perlin(self) -> f32 {
        panic!("type did not implement perlin noise");
    }
    fn simplex(self) -> f32 {
        panic!("type did not implement simplex noise");
    }
    fn voronoi(self, #[allow(unused)] smooth: f32) -> f32 {
        panic!("type did not implement voronoi");
    }
    fn worley(self) -> f32 {
        panic!("type did not implement worley noise");
    }

    fn value_tile(self, #[allow(unused)] scale: Self) -> f32 {
        panic!("type did not implement tiled value noise");
    }
    fn perlin_tile(self, #[allow(unused)] scale: Self) -> f32 {
        panic!("type did not implement tiled perlin noise");
    }
    fn voronoi_tile(self, #[allow(unused)] smooth: f32, #[allow(unused)] scale: Self) -> f32 {
        panic!("type did not implement tiled voronoi");
    }
    fn worley_tile(self, #[allow(unused)] scale: Self) -> f32 {
        panic!("type did not implement tiled worley noise");
    }

    fn fbm(self, f: impl Fn(Self) -> f32, oct: u32) -> f32 {
        self.fbm_fn(
            |p, a| {
                let n = f(*p);
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
    fn hash(mut self) -> f32 {
        self *= 3141592653.0;
        let u32b = self.to_bits();
        (u32b.wrapping_mul(u32b).wrapping_mul(3141592653)) as f32 / (u32::MAX as f32)
    }

    fn value(self) -> f32 {
        let fl = self.abs().floor();
        let fr = self.abs().fract();
        fl.hash().lerp((fl + 1.0).hash(), fr.smooth())
    }
}

impl Noise for Vec2 {
    fn hash(self) -> f32 {
        let ux = (self.x * 141421356.0).to_bits();
        let uy = (self.y * 2718281828.0).to_bits();
        ((ux ^ uy) * 3141592653u32) as f32 / u32::MAX as f32
    }

    fn hash2(self) -> Vec2 {
        let ux = (self.x * 141421356.0).to_bits();
        let uy = (self.y * 2718281828.0).to_bits();
        return Vec2::from((ux ^ uy) * Vec2u::new(3141592653, 1618033988)) / u32::MAX as f32;
    }

    fn value(self) -> f32 {
        let i = self.floor();
        let u = self.fract().smooth();
        let res = i
            .hash()
            .lerp((i + Vec2::X).hash(), u.x)
            .lerp((i + Vec2::Y).hash().lerp((i + 1.0).hash(), u.x), u.y);
        res
    }

    fn value_tile(self, scale: Self) -> f32 {
        let i = self.floor();
        let u = self.fract().smooth();
        let hash = |v: Vec2| (v / scale).fract().hash();
        let res = hash(i)
            .lerp(hash(i + Vec2::X), u.x)
            .lerp(hash(i + Vec2::Y).lerp(hash(i + 1.0), u.x), u.y);
        res
    }

    fn perlin(self) -> f32 {
        let i = self.floor();
        let f = self - i;
        let u = f * f * f * (10.0 + f * (6.0 * f - 15.0));
        let hash_dir = |v: Self| (v.hash2() - 0.5).norm();
        let a = hash_dir(i + Self::ZERO).dot(f - Self::ZERO);
        let b = hash_dir(i + Self::X).dot(f - Self::X);
        let c = hash_dir(i + Self::Y).dot(f - Self::Y);
        let d = hash_dir(i + Self::XY).dot(f - Self::XY);
        return a.lerp(b, u.x).lerp(c.lerp(d, u.x), u.y) * 0.7 + 0.5;
    }

    fn perlin_tile(self, scale: Self) -> f32 {
        let i = self.floor();
        let f = self - i;
        let u = f * f * f * (10.0 + f * (6.0 * f - 15.0));
        let hash_dir = |v: Self| ((v / scale).fract().hash2() - 0.5).norm();
        let a = hash_dir(i + Self::ZERO).dot(f - Self::ZERO);
        let b = hash_dir(i + Self::X).dot(f - Self::X);
        let c = hash_dir(i + Self::Y).dot(f - Self::Y);
        let d = hash_dir(i + Self::XY).dot(f - Self::XY);
        return a.lerp(b, u.x).lerp(c.lerp(d, u.x), u.y) * 0.7 + 0.5;
    }

    fn simplex(self) -> f32 {
        let i = (self + (self.x + self.y) * 0.366025).floor();
        let a = self - i + (i.x + i.y) * 0.211324;
        let m = a.y.step(a.x);
        let o = Vec2::new(m, 1.0 - m);
        let b = a - o + 0.211324;
        let c = a - 0.577351;
        let mut h = Vec3::ZERO.max(0.5 - Vec3::new(a.len2(), b.len2(), c.len2()));
        h *= h;
        let n = h
            * h
            * Vec3::new(
                a.dot(i.hash2() - 0.5),
                b.dot((i + o).hash2() - 0.5),
                c.dot((i + 1.0).hash2() - 0.5),
            );
        return n.dot(Vec3::splat(70.0)) + 0.5;
    }

    fn voronoi(self, smooth: f32) -> f32 {
        let hash32 = |p: Vec2| {
            let mut p3 = (p.xyx() / Vec3::new(0.1031, 0.1030, 0.0973)).fract();
            p3 += p3.dot(p3.yxz() + 33.33);
            return ((p3.xxy() + p3.yzz()) * p3.zyx()).fract();
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
                let d = (c - f + o.xy()).len();
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
                let c = p - c - ((id + c) / scale).fract().hash();
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

    fn hash2(self) -> Vec2 {
        let u = (self * Vec3::new(141421356.0, 2718281828.0, 1618033988.0)).to_bits();
        return Vec2::from((u.x ^ u.y ^ u.z) * Vec2u::new(1732050807, 2645751311))
            / u32::MAX as f32;
    }

    fn hash3(self) -> Vec3 {
        let u = (self * Vec3::new(141421356.0, 2718281828.0, 1618033988.0)).to_bits();
        return Vec3::from((u.x ^ u.y ^ u.z) * Vec3u::new(1732050807, 2645751311, 3316624790))
            / u32::MAX as f32;
    }

    fn value(self) -> f32 {
        let i = self.floor();
        let u = self.fract().smooth();
        i.hash()
            .lerp((i + Vec3::X).hash(), u.x)
            .lerp((i + Vec3::Y).hash().lerp((i + Vec3::XY).hash(), u.x), u.y)
            .lerp(
                (i + Vec3::Z)
                    .hash()
                    .lerp((i + Vec3::XZ).hash(), u.x)
                    .lerp((i + Vec3::YZ).hash().lerp((i + Vec3::XYZ).hash(), u.x), u.y),
                u.z,
            )
    }

    fn value_tile(self, scale: Self) -> f32 {
        let i = self.floor();
        let u = self.fract().smooth();
        let hash = |v: Self| (v / scale).fract().hash();
        hash(i)
            .lerp(hash(i + Vec3::X), u.x)
            .lerp(hash(i + Vec3::Y).lerp(hash(i + Vec3::XY), u.x), u.y)
            .lerp(
                hash(i + Vec3::Z)
                    .lerp(hash(i + Vec3::XZ), u.x)
                    .lerp(hash(i + Vec3::YZ).lerp(hash(i + Vec3::XYZ), u.x), u.y),
                u.z,
            )
    }

    fn perlin(self) -> f32 {
        let i = self.floor();
        let f = self - i;
        let u = f * f * f * (10.0 + f * (6.0 * f - 15.0));
        let hash_dir = |v: Self| (v.hash3() - 0.5).norm();
        let a0 = hash_dir(i + Self::ZERO).dot(f - Self::ZERO);
        let b0 = hash_dir(i + Self::X).dot(f - Self::X);
        let c0 = hash_dir(i + Self::Y).dot(f - Self::Y);
        let d0 = hash_dir(i + Self::XY).dot(f - Self::XY);
        let a1 = hash_dir(i + Self::Z).dot(f - Self::Z);
        let b1 = hash_dir(i + Self::XZ).dot(f - Self::XZ);
        let c1 = hash_dir(i + Self::YZ).dot(f - Self::YZ);
        let d1 = hash_dir(i + Self::XYZ).dot(f - Self::XYZ);
        let z0 = a0.lerp(b0, u.x).lerp(c0.lerp(d0, u.x), u.y);
        let z1 = a1.lerp(b1, u.x).lerp(c1.lerp(d1, u.x), u.y);
        return z0.lerp(z1, u.z) * 0.7 + 0.5;
    }

    fn perlin_tile(self, scale: Self) -> f32 {
        let i = self.floor();
        let f = self - i;
        let u = f * f * f * (10.0 + f * (6.0 * f - 15.0));
        let hash_dir = |v: Self| ((v / scale).fract().hash3() - 0.5).norm();
        let a0 = hash_dir(i + Self::ZERO).dot(f - Self::ZERO);
        let b0 = hash_dir(i + Self::X).dot(f - Self::X);
        let c0 = hash_dir(i + Self::Y).dot(f - Self::Y);
        let d0 = hash_dir(i + Self::XY).dot(f - Self::XY);
        let a1 = hash_dir(i + Self::Z).dot(f - Self::Z);
        let b1 = hash_dir(i + Self::XZ).dot(f - Self::XZ);
        let c1 = hash_dir(i + Self::YZ).dot(f - Self::YZ);
        let d1 = hash_dir(i + Self::XYZ).dot(f - Self::XYZ);
        let z0 = a0.lerp(b0, u.x).lerp(c0.lerp(d0, u.x), u.y);
        let z1 = a1.lerp(b1, u.x).lerp(c1.lerp(d1, u.x), u.y);
        return z0.lerp(z1, u.z) * 0.7 + 0.5;
    }

    fn simplex(self) -> f32 {
        let s = (self + self.dot(Vec3::splat(1.0 / 3.0))).floor();
        let x = self - s + s.dot(Vec3::splat(1.0 / 6.0));
        let e = Vec3::ZERO.step(x - x.yzx());
        let i1 = e * (1.0 - e.zxy());
        let i2 = 1.0 - e.zxy() * (1.0 - e);
        let x1 = x - i1 + 1.0 / 6.0;
        let x2 = x - i2 + 1.0 / 3.0;
        let x3 = x - 0.5;
        let mut w = Vec4::new(x.len2(), x1.len2(), x2.len2(), x3.len2());
        w = Vec4::ZERO.max(0.6 - w);
        let mut d = Vec4::new(
            x.dot(s.hash3() - 0.5),
            x1.dot((s + i1).hash3() - 0.5),
            x2.dot((s + i2).hash3() - 0.5),
            x3.dot((s + 1.0).hash3() - 0.5),
        );
        w *= w;
        w *= w;
        d *= w;
        return d.dot(Vec4::splat(26.0)) + 0.5;
    }

    fn worley(self) -> f32 {
        let i = self.floor();
        let p = self - i;
        let mut w = 1e9f32;
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let c = Vec3::new(x as f32, y as f32, z as f32);
                    let c = p - c - (i + c).hash();
                    w = w.min(c.len2());
                }
            }
        }
        return 1.0 - w.sqrt();
    }

    fn worley_tile(self, scale: Self) -> f32 {
        let i = self.floor();
        let p = self - i;
        let mut w = 1e9f32;
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let c = Self::new(x as f32, y as f32, z as f32);
                    let c = p - c - ((i + c) / scale).fract().hash();
                    w = w.min(c.len2());
                }
            }
        }
        return 1.0 - w.sqrt();
    }
}
