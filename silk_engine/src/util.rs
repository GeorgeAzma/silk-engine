pub(crate) mod print;

mod bmp;
mod buddy_alloc;
mod contain_range;
mod cooldown;
mod image_loader;
mod mem;
mod noise;
mod qoi;
mod rand;
mod reader;
mod tracked;
mod ttf;
mod vec;
mod writer;

pub(crate) use buddy_alloc::BuddyAlloc;
pub(crate) use contain_range::ContainRange;
pub(crate) use ttf::{GlyphData, Ttf};

pub use bmp::Bmp;
pub use cooldown::Cooldown;
pub use image_loader::{ImageData, ImageFormat, ImageLoader};
pub use mem::Mem;
pub use noise::Noise;
pub use qoi::Qoi;
pub use rand::Rand;
pub use reader::{Reader, ReaderBe};
pub use tracked::Tracked;
pub use vec::{Bezier, ExtraFns, Vec2, Vec2u, Vec3, Vectorf, Vectoru};
pub use writer::Writer;

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

pub fn cast_slice<T: ?Sized, U>(p: &T) -> &[U] {
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
