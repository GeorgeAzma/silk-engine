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

impl std::fmt::Debug for Mem {
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

impl std::fmt::Display for Mem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
