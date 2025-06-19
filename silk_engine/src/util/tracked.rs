use std::ops::{Deref, DerefMut};

pub struct Tracked<T> {
    data: T,
    dirty: bool,
}

impl<T> Tracked<T> {
    pub const fn new(data: T) -> Self {
        Self { data, dirty: false }
    }

    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub const fn reset(&mut self) {
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
