use std::mem::ManuallyDrop;

pub mod alloc;
pub mod bmp;
pub mod bounding_range;
pub mod cooldown;
pub mod font;
pub mod image_loader;
pub mod mem;
pub mod packer;
pub mod print;
pub mod qoi;
pub mod rand;
pub mod reader;
pub mod tracked;
pub mod ttf;
pub mod vec;
pub mod wav;
pub mod writer;

pub fn cast_vec<T, U>(v: Vec<T>) -> Vec<U> {
    use std::mem::size_of;
    assert_eq!(
        size_of::<T>() * v.len() % size_of::<U>(),
        0,
        "size mismatch"
    );

    let len = (size_of::<T>() * v.len()) / size_of::<U>();
    let cap = (size_of::<T>() * v.capacity()) / size_of::<U>();

    let mut v = ManuallyDrop::new(v);
    let ptr = v.as_mut_ptr() as *mut U;

    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

pub fn cast_slice<T, U>(p: &[T]) -> &[U] {
    unsafe {
        std::slice::from_raw_parts(
            p.as_ptr() as *const U,
            std::mem::size_of_val(p) / size_of::<U>(),
        )
    }
}

pub fn to_slice<T: ?Sized, U>(p: &T) -> &[U] {
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
