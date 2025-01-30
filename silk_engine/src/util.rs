pub(crate) mod print;

mod bmp;
mod buddy_alloc;
mod contain_range;
mod cooldown;
mod image_loader;
mod mem;
mod qoi;
mod rand;
mod reader;
mod tracked;
mod ttf;
mod vec;
mod writer;

pub(crate) use bmp::Bmp;
pub(crate) use buddy_alloc::BuddyAlloc;
pub(crate) use contain_range::ContainRange;
pub(crate) use image_loader::{ImageData, ImageFormat, ImageLoader};
pub(crate) use qoi::Qoi;
pub(crate) use ttf::Ttf;

pub use cooldown::Cooldown;
pub use mem::Mem;
pub use rand::Rand;
pub use reader::{Reader, ReaderBe};
pub use tracked::Tracked;
pub use vec::{Funcs, Vec2, Vec3};
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
