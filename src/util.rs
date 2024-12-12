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
