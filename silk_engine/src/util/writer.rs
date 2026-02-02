use super::to_slice;

pub struct Writer {
    idx: usize,
    bytes: Box<[u8]>,
}

impl Writer {
    pub fn new(size: usize) -> Self {
        Self {
            idx: 0,
            bytes: vec![0u8; size].into_boxed_slice(),
        }
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes.into_vec()
    }

    pub fn goto(&mut self, idx: usize) {
        self.idx = idx;
    }

    pub fn skip(&mut self, bytes: usize) {
        self.idx += bytes;
    }

    pub fn write<T: ?Sized>(&mut self, val: &T) {
        let size = size_of_val(val);
        self.bytes[self.idx..][..size].copy_from_slice(to_slice(val));
        self.idx += size;
    }

    pub fn write8(&mut self, byte: u8) {
        self.bytes[self.idx] = byte;
        self.idx += 1;
    }

    pub fn write16(&mut self, word: u16) {
        self.write(&word.to_le_bytes())
    }

    pub fn write32(&mut self, dword: u32) {
        self.write(&dword.to_le_bytes())
    }

    pub fn write64(&mut self, qword: u64) {
        self.write(&qword.to_le_bytes())
    }
}
