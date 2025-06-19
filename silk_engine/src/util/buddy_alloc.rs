#[derive(Clone, Debug)]
pub struct BuddyAlloc {
    size: usize,
    /// 2^i -> [offsets]
    free_lists: Vec<Vec<usize>>,
}

impl BuddyAlloc {
    pub fn new(size: usize) -> Self {
        let size2 = size.next_power_of_two();
        let len = size2.trailing_zeros() as usize + 1;
        let mut free_lists = vec![Vec::new(); len];
        free_lists[len - 1].push(0);
        Self { size, free_lists }
    }

    /// `O(log N)`, where N is pool size\
    /// non pow2 sizes are rounded up causing fragmentation\
    /// finds smallest block that can contain size\
    /// if block is bigger than size * 2, splits it\
    /// ### Returns
    /// - offset of newly alloced memory
    /// - `usize::MAX` when out of space
    pub fn alloc(&mut self, size: usize) -> usize {
        let size2 = size.next_power_of_two();
        let mut i = size2.trailing_zeros() as usize;
        if i >= self.free_lists.len() {
            return usize::MAX;
        } else {
            while i < self.free_lists.len() && self.free_lists[i].is_empty() {
                i += 1;
            }
            if i >= self.free_lists.len() {
                return usize::MAX;
            }
        }
        let offset = self.free_lists[i].pop().unwrap();
        let mut current_size = 1 << i;
        while current_size > size2 {
            current_size /= 2;
            self.free_lists[i - 1].push(offset + current_size);
            i -= 1;
        }
        offset
    }

    /// `O(log N)`, where `N` is allocations\
    /// worst case `O(n)` where n is allocations
    pub fn dealloc(&mut self, offset: usize, size: usize) {
        let size2 = size.next_power_of_two();
        self.merge(size2, offset);
    }

    /// grow and add/extend free ranges\
    /// or shrink and remove outside ranges
    pub fn resize(&mut self, new_size: usize) {
        self.size = new_size;
        let new_size2 = new_size.next_power_of_two();
        let new_len = new_size2.trailing_zeros() as usize + 1;
        let old_len = self.free_lists.len();

        match new_len.cmp(&old_len) {
            // shrink and remove
            std::cmp::Ordering::Less => {
                for fl in self.free_lists.iter().skip(new_len) {
                    if fl.contains(&0) {
                        self.free_lists[new_len - 1].push(0);
                        break;
                    }
                }
                self.free_lists.resize(new_len, Vec::new());
                for (i, fl) in self.free_lists[..new_len].iter_mut().enumerate() {
                    fl.retain(|x| *x + (1 << i) <= new_size2);
                }
            }
            std::cmp::Ordering::Greater => {
                self.free_lists.resize(new_len, Vec::new());
                if self.free_lists[old_len - 1] == vec![0] {
                    self.free_lists[old_len - 1] = vec![];
                    self.free_lists[new_len - 1] = vec![0];
                } else {
                    for i in old_len..new_len {
                        self.free_lists[i - 1].push(1 << (i - 1));
                    }
                }
            }
            _ => {}
        }
    }

    #[allow(unused)]
    pub fn len(&self) -> usize {
        self.size
    }

    /// `O(log N)`, where N is pool size
    /// worst case `O(n)` where n is allocations
    /// checks if buddy is free and merges it
    /// otherwise adds current block to free list
    fn merge(&mut self, size: usize, offset: usize) {
        let i = size.trailing_zeros() as usize;
        if let Some(pos) = self.free_lists[i].iter().position(|&x| x == offset ^ size) {
            self.free_lists[i].swap_remove(pos);
            self.merge(size * 2, offset & !size);
        } else {
            self.free_lists[i].push(offset);
        }
    }
}
