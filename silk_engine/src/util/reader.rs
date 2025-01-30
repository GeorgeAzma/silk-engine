pub struct Reader<'a> {
    idx: usize,
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { idx: 0, bytes }
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn goto(&mut self, idx: usize) {
        self.idx = idx;
    }

    pub fn skip(&mut self, bytes: usize) {
        self.idx += bytes;
    }

    pub fn read_arr<const N: usize>(&mut self) -> [u8; N] {
        let array: [u8; N] = *self.bytes[self.idx..][..N].as_array::<N>().unwrap();
        self.idx += N;
        array
    }

    pub fn read(&mut self, num_bytes: usize) -> &[u8] {
        let bytes = &self.bytes[self.idx..][..num_bytes];
        self.idx += num_bytes;
        bytes
    }

    pub fn read8(&mut self) -> u8 {
        let byte = self.bytes[self.idx];
        self.idx += 1;
        byte
    }

    pub fn read16(&mut self) -> u16 {
        let bytes = self.read_arr::<2>();
        u16::from_le_bytes(bytes)
    }

    pub fn read32(&mut self) -> u32 {
        let bytes = self.read_arr::<4>();
        u32::from_le_bytes(bytes)
    }

    pub fn read64(&mut self) -> u64 {
        let bytes = self.read_arr::<8>();
        u64::from_le_bytes(bytes)
    }
}

pub struct ReaderBe<'a> {
    idx: usize,
    bytes: &'a [u8],
}

impl<'a> ReaderBe<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { idx: 0, bytes }
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn goto(&mut self, idx: usize) {
        self.idx = idx;
    }

    pub fn skip(&mut self, bytes: usize) {
        self.idx += bytes;
    }

    pub fn read_arr<const N: usize>(&mut self) -> [u8; N] {
        let array: [u8; N] = *self.bytes[self.idx..][..N].as_array::<N>().unwrap();
        self.idx += N;
        array
    }

    pub fn read(&mut self, num_bytes: usize) -> &[u8] {
        let bytes = &self.bytes[self.idx..][..num_bytes];
        self.idx += num_bytes;
        bytes
    }

    pub fn read8(&mut self) -> u8 {
        let byte = self.bytes[self.idx];
        self.idx += 1;
        byte
    }

    pub fn read16(&mut self) -> u16 {
        let bytes = self.read_arr::<2>();
        u16::from_be_bytes(bytes)
    }

    pub fn read32(&mut self) -> u32 {
        let bytes = self.read_arr::<4>();
        u32::from_be_bytes(bytes)
    }

    pub fn read64(&mut self) -> u64 {
        let bytes = self.read_arr::<8>();
        u64::from_be_bytes(bytes)
    }
}
