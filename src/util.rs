#[macro_export]
macro_rules! expose_method {
    ($member:ident.$method:ident($($arg_name:ident : $arg_type:ty),*) -> $ret:ty) => {
        #[inline]
        pub fn $method(&self, $($arg_name: $arg_type),*) -> $ret {
            self.$member.$method($($arg_name),*)
        }
    }
}

#[macro_export]
macro_rules! expose_methods {
    ($member:ident.[$($method:ident),*]($arg_name:ident : $arg_type:ty) -> $ret:ty) => {
        $(
            expose_method!($member.$method($arg_name: $arg_type) -> $ret);
        )*
    }
}

pub fn cast_slice<T>(slice: &[T]) -> &[u8] {
    assert!(size_of::<T>() > 0, "Cannot cast a zero-sized type");
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, size_of_val(slice)) }
}

pub fn cast_slice_to<A, B>(slice: &[A]) -> &[B] {
    assert!(size_of::<A>() > 0, "Cannot cast a zero-sized type");
    assert!(size_of::<B>() > 0, "Cannot cast to zero-sized type");
    unsafe {
        std::slice::from_raw_parts(
            slice.as_ptr() as *const B,
            size_of_val(slice) / size_of::<B>(),
        )
    }
}

#[cfg(not(debug_assertions))]
pub struct ScopeTime;

#[cfg(not(debug_assertions))]
impl ScopeTime {
    pub fn new(name: &str) -> Self {
        Self
    }
}

#[cfg(debug_assertions)]
pub struct ScopeTime {
    start: std::time::Instant,
    name: String,
}

#[cfg(debug_assertions)]
impl ScopeTime {
    pub fn new(name: &str) -> Self {
        Self {
            start: std::time::Instant::now(),
            name: name.to_string(),
        }
    }
}

#[cfg(debug_assertions)]
impl Drop for ScopeTime {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        crate::print::log(&format!("{}: {:?}", self.name, elapsed));
    }
}
