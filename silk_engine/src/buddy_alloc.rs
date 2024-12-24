#[derive(Clone)]
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

    /// `O(log N)`, where N is pool size
    /// non pow2 sizes are rounded up causing fragmentation
    /// finds smallest block that can contain size
    /// if block is bigger than size * 2, splits it
    /// returns usize::MAX when out of space
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

    /// `O(log N)`, where `N` is allocations
    /// worst case `O(n)` where n is allocations
    pub fn dealloc(&mut self, offset: usize, size: usize) {
        let size2 = size.next_power_of_two();
        self.merge(size2, offset);
    }

    /// shrink and remove outside ranges
    /// or grow and add/extend free ranges
    /// TODO: needs testing, might have heap corruption rarely
    pub fn resize(&mut self, new_size: usize) {
        self.size = new_size;
        let new_size2 = new_size.next_power_of_two();
        let new_len = new_size2.trailing_zeros() as usize + 1;
        let old_len = self.free_lists.len();

        match new_len.cmp(&old_len) {
            // shrink and remove
            std::cmp::Ordering::Less => {
                self.free_lists.resize(new_len, Vec::new());
                let mut empty = true;
                for (i, fl) in self.free_lists[..new_len - 1].iter_mut().enumerate() {
                    fl.retain(|x| *x + (1 << i) <= new_size2);
                    empty &= fl.is_empty();
                }
                if empty {
                    if self.free_lists[new_len - 1].is_empty() {
                        self.free_lists[new_len - 1].push(0);
                    } else if self.free_lists[new_len - 1][0] == 1 << (new_len - 1) {
                        self.free_lists[new_len - 1] = vec![];
                    }
                }
            }
            std::cmp::Ordering::Greater => {
                self.free_lists.resize(new_len, Vec::new());
                if self.free_lists[old_len - 1] == vec![0] {
                    self.free_lists[old_len - 1] = vec![];
                    self.free_lists[new_len - 1] = vec![0];
                } else {
                    let empty = self.free_lists[..old_len].iter().all(|x| x.is_empty());
                    if !empty {
                        self.free_lists[old_len].push(1 << old_len);
                    } else {
                        self.free_lists[old_len - 1].push(1 << (old_len - 1));
                    }
                }
            }
            _ => {}
        }
    }

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
