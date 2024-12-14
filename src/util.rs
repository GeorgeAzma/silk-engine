use core::fmt;

#[macro_export]
macro_rules! expose {
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

pub struct Mem {
    bytes: u64,
}

impl Mem {
    pub fn b(bytes: u64) -> Self {
        Self { bytes }
    }

    pub fn kb(kb: u64) -> Self {
        Self { bytes: kb << 10 }
    }

    pub fn mb(mb: u64) -> Self {
        Self { bytes: mb << 20 }
    }

    pub fn gb(gb: u64) -> Self {
        Self { bytes: gb << 30 }
    }

    pub fn tb(tb: u64) -> Self {
        Self { bytes: tb << 40 }
    }

    pub fn str(s: &str) -> Self {
        Self {
            bytes: s.parse().unwrap(),
        }
    }

    pub fn as_bytes(&self) -> u64 {
        self.bytes
    }

    pub fn as_b(&self) -> u64 {
        self.bytes
    }

    pub fn as_kb(&self) -> u64 {
        self.bytes >> 10
    }

    pub fn as_mb(&self) -> u64 {
        self.bytes >> 20
    }

    pub fn as_gb(&self) -> u64 {
        self.bytes >> 30
    }

    pub fn as_tb(&self) -> u64 {
        self.bytes >> 40
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
