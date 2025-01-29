use core::fmt;
use std::{
    ops::{Deref, DerefMut},
    time::{Duration, Instant},
};

#[macro_export]
macro_rules! expose {
    (($member:expr).$method:ident($($arg_name:ident : $arg_type:ty),*) -> $ret:ty) => {
        #[inline]
        pub fn $method(&self, $($arg_name: $arg_type),*) -> $ret {
            self.$member.$method($($arg_name),*)
        }
    };
    (($member:expr).[$($method:ident),*]($arg_name:ident : $arg_type:ty) -> $ret:ty) => {
        $(
            #[inline]
            pub fn $method(&self, $arg_name: $arg_type) -> $ret {
                self.$member.$method($arg_name)
            }
        )*
    };
    ($member:ident.$method:ident($($arg_name:ident : $arg_type:ty),*) -> $ret:ty) => {
        #[inline]
        pub fn $method(&self, $($arg_name: $arg_type),*) -> $ret {
            self.$member.$method($($arg_name),*)
        }
    };
    ($member:ident.[$($method:ident),*]($arg_name:ident : $arg_type:ty) -> $ret:ty) => {
        $(
            #[inline]
            pub fn $method(&self, $arg_name: $arg_type) -> $ret {
                self.$member.$method($arg_name)
            }
        )*
    };
}

pub fn as_slice<T: ?Sized, U>(p: &T) -> &[U] {
    unsafe {
        std::slice::from_raw_parts((p as *const T) as *const U, size_of_val(p) / size_of::<U>())
    }
}

pub fn from_slice<T, U>(p: &[U]) -> &T {
    assert_eq!(
        size_of::<T>(),
        size_of_val(p),
        "slice must have same size as struct for casting"
    );
    unsafe { &*(p.as_ptr() as *const T) }
}

pub fn from_bytes<T>(p: &[u8]) -> &T {
    from_slice::<T, u8>(p)
}

pub struct Reader<'a, const BE: bool = false> {
    idx: usize,
    bytes: &'a [u8],
}

impl<'a, const BE: bool> Reader<'a, BE> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { idx: 0, bytes }
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn goto(&mut self, idx: usize) {
        self.idx = idx;
    }

    pub fn skip(&mut self, bytes: usize) {
        self.idx += bytes;
    }

    pub fn read_arr<const N: usize>(&mut self) -> [u8; N] {
        let array: [u8; N] = *self.bytes[self.idx..][..N].as_array::<N>().unwrap();
        self.idx += N;
        array
    }

    pub fn read(&mut self) -> u8 {
        let byte = self.bytes[self.idx];
        self.idx += 1;
        byte
    }

    pub fn read16(&mut self) -> u16 {
        let bytes = self.read_arr::<2>();
        if BE {
            u16::from_be_bytes(bytes)
        } else {
            u16::from_le_bytes(bytes)
        }
    }

    pub fn read32(&mut self) -> u32 {
        let bytes = self.read_arr::<4>();
        if BE {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        }
    }

    pub fn read64(&mut self) -> u64 {
        let bytes = self.read_arr::<8>();
        if BE {
            u64::from_be_bytes(bytes)
        } else {
            u64::from_le_bytes(bytes)
        }
    }
}

pub struct Mem {
    bytes: usize,
}

impl Mem {
    pub const fn b(bytes: usize) -> Self {
        Self { bytes }
    }

    pub const fn kb(kb: usize) -> Self {
        Self { bytes: kb << 10 }
    }

    pub const fn mb(mb: usize) -> Self {
        Self { bytes: mb << 20 }
    }

    pub const fn gb(gb: usize) -> Self {
        Self { bytes: gb << 30 }
    }

    pub const fn tb(tb: usize) -> Self {
        Self { bytes: tb << 40 }
    }

    pub fn str(s: &str) -> Self {
        Self {
            bytes: s.parse().unwrap(),
        }
    }

    pub const fn as_bytes(&self) -> usize {
        self.bytes
    }

    pub const fn as_b(&self) -> usize {
        self.bytes
    }

    pub const fn as_kb(&self) -> usize {
        self.bytes >> 10
    }

    pub const fn as_mb(&self) -> usize {
        self.bytes >> 20
    }

    pub const fn as_gb(&self) -> usize {
        self.bytes >> 30
    }

    pub const fn as_tb(&self) -> usize {
        self.bytes >> 40
    }
}

impl std::ops::Deref for Mem {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl fmt::Debug for Mem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let by = self.bytes;
        let kb = by / 1024;
        let mb = kb / 1024;
        let gb = mb / 1024;
        let tb = gb / 1024;
        if tb > 0 {
            write!(f, "{} TiB", gb)
        } else if gb > 0 {
            write!(f, "{} GiB", gb)
        } else if mb > 0 {
            write!(f, "{} MiB", mb)
        } else if kb > 0 {
            write!(f, "{} KiB", kb)
        } else {
            write!(f, "{} B", by)
        }
    }
}

impl fmt::Display for Mem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

pub struct Cooldown {
    timer: Instant,
    pub delay: Duration,
}

#[allow(unused)]
impl Cooldown {
    pub fn new(delay: Duration) -> Self {
        Self {
            timer: Instant::now(),
            delay,
        }
    }

    pub fn sec(secs: f32) -> Self {
        Self::new(Duration::from_secs_f32(secs))
    }

    pub fn ms(ms: u32) -> Self {
        Self::new(Duration::from_millis(ms as u64))
    }

    pub fn us(us: u32) -> Self {
        Self::new(Duration::from_micros(us as u64))
    }

    pub fn ns(ns: u32) -> Self {
        Self::new(Duration::from_nanos(ns as u64))
    }

    pub fn ready(&self) -> bool {
        self.dt() >= self.delay
    }

    pub fn dt(&self) -> Duration {
        self.timer.elapsed()
    }

    pub fn reset(&mut self) {
        self.timer = Instant::now();
    }

    pub fn next(&mut self) {
        self.timer += self.delay;
    }
}

pub struct Tracked<T> {
    data: T,
    dirty: bool,
}

impl<T> Tracked<T> {
    pub fn new(data: T) -> Self {
        Self { data, dirty: false }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn reset(&mut self) {
        self.dirty = false;
    }
}

impl<T> Deref for Tracked<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Tracked<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        &mut self.data
    }
}

pub trait Lerp {
    fn lerp(self, b: Self, k: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, b: f32, k: f32) -> f32 {
        self + (b - self) * k
    }
}

pub trait Bezier {
    fn bezier(self, b: f32, c: f32, t: f32) -> Self;
}

impl Bezier for f32 {
    fn bezier(self, b: f32, c: f32, t: f32) -> f32 {
        let a = self;
        t * (t * (c - 2.0 * b + a) + 2.0 * (b - a)) + a
    }
}
