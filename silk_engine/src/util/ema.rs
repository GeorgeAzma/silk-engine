use std::{
    collections::HashMap,
    panic::Location,
    sync::{LazyLock, Mutex},
};

static EMA_F32: LazyLock<Mutex<HashMap<usize, f32>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static EMA_F64: LazyLock<Mutex<HashMap<usize, f64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Build a stable `usize` key from the caller's source location.
/// `Location::file()` is a `&'static str`, so its pointer is constant
/// for the entire program; line + column disambiguate call sites.
#[track_caller]
fn caller_key() -> usize {
    let loc = Location::caller();
    let mut h = loc.file().as_ptr() as usize;
    h = h.wrapping_mul(0x9e3779b9).wrapping_add(loc.line() as usize);
    h = h.wrapping_mul(0x9e3779b9).wrapping_add(loc.column() as usize);
    h
}

pub trait Ema {
    #[track_caller]
    fn ema(&self, alpha: Self) -> Self;
}

impl Ema for f32 {
    fn ema(&self, alpha: Self) -> Self {
        let mut map = EMA_F32.lock().unwrap();
        let ema = map.entry(caller_key()).or_insert(*self);
        *ema = alpha * self + (1.0 - alpha) * *ema;
        *ema
    }
}

impl Ema for f64 {
    fn ema(&self, alpha: Self) -> Self {
        let mut map = EMA_F64.lock().unwrap();
        let ema = map.entry(caller_key()).or_insert(*self);
        *ema = alpha * self + (1.0 - alpha) * *ema;
        *ema
    }
}