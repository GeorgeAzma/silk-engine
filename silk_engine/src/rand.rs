pub trait Rand {
    fn rand(self) -> Self;
}

impl Rand for u8 {
    fn rand(mut self) -> Self {
        self = self.wrapping_add(0x5D);
        let mut x = self;
        x = (x ^ (x >> 4)).wrapping_mul(0x1D);
        x = (x ^ (x >> 3)).wrapping_mul(0x3B);
        x ^= x >> 3;
        x
    }
}

impl Rand for u16 {
    fn rand(mut self) -> Self {
        self = self.wrapping_add(0x9E3D);
        let mut x = self;
        x = (x ^ (x >> 8)).wrapping_mul(0x2F1D);
        x = (x ^ (x >> 7)).wrapping_mul(0x623B);
        x ^= x >> 7;
        x
    }
}

impl Rand for u32 {
    fn rand(mut self) -> Self {
        self = self.wrapping_add(0x9E3779B9);
        let mut x = self;
        x = (x ^ (x >> 16)).wrapping_mul(0x21F0AAAD);
        x = (x ^ (x >> 15)).wrapping_mul(0x735A2D97);
        x ^= x >> 15;
        x
    }
}

impl Rand for u64 {
    fn rand(mut self) -> Self {
        self = self.wrapping_add(0x9E3779B97F4A7C15);
        let mut x = self;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
        x ^ (x >> 31)
    }
}

impl Rand for f32 {
    fn rand(mut self) -> Self {
        self *= 3141592653.0;
        let u32b = self.to_bits();
        return (u32b.wrapping_mul(u32b).wrapping_mul(3141592653)) as f32 / (u32::MAX as f32);
    }
}
