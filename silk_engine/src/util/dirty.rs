use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct Dirty<T> {
    data: T,
    dirty: bool,
}

impl<T> Dirty<T> {
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

impl<T> Deref for Dirty<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Dirty<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty = true;
        &mut self.data
    }
}
