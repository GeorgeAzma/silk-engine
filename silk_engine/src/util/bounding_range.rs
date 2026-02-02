use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    ops::Range,
};

/// tracks bounding range of smaller mapped ranges
#[derive(Clone)]
pub struct BoundingRange {
    /// keeps track of bounding range min
    mins: BinaryHeap<Reverse<usize>>,
    /// keeps track of bounding range max
    maxs: BinaryHeap<usize>,
    /// used for lazy deletion of mapped range min
    del_mins: HashMap<usize, usize>,
    /// used for lazy deletion of mapped range max
    del_maxs: HashMap<usize, usize>,
}

impl Default for BoundingRange {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundingRange {
    pub fn new() -> Self {
        Self {
            mins: Default::default(),
            maxs: Default::default(),
            del_mins: Default::default(),
            del_maxs: Default::default(),
        }
    }

    pub fn add(&mut self, start: usize, end: usize) {
        self.mins.push(Reverse(start));
        self.maxs.push(end);
    }

    /// removes mapped range lazily, actual remove happens in range() function to amortize binary heap `O(log n)` removal cost
    pub fn remove(&mut self, start: usize, end: usize) {
        *self.del_mins.entry(start).or_insert(0) += 1;
        *self.del_maxs.entry(end).or_insert(0) += 1;
    }

    /// `range()` executes accumulated lazy deletions and returns final bounding range
    pub fn range(&mut self) -> Range<usize> {
        let mut start = 0;
        while let Some(Reverse(min)) = self.mins.peek() {
            if let Some(count) = self.del_mins.get(min) {
                if *count > 0 {
                    if *count == 1 {
                        self.del_mins.remove(min);
                    } else {
                        self.del_mins.insert(*min, count - 1);
                    }
                    self.mins.pop();
                } else {
                    start = *min;
                    break;
                }
            } else {
                start = *min;
                break;
            }
        }

        let mut end = 0;
        while let Some(max) = self.maxs.peek() {
            if let Some(count) = self.del_maxs.get(max) {
                if *count > 0 {
                    if *count == 1 {
                        self.del_maxs.remove(max);
                    } else {
                        self.del_maxs.insert(*max, count - 1);
                    }
                    self.maxs.pop();
                } else {
                    self.del_maxs.remove(max);
                    end = *max;
                    break;
                }
            } else {
                end = *max;
                break;
            }
        }
        start..end
    }
}
