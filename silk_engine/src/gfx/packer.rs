#[derive(Clone, Copy)]
pub struct Rect(u64);

impl Rect {
    pub fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self(((w as u64) << 48) | ((h as u64) << 32) | ((x as u64) << 16) | y as u64)
    }

    pub fn xywh(&self) -> (u16, u16, u16, u16) {
        let r = self.0;
        (
            (r >> 16) as u16,
            (r/****/) as u16,
            (r >> 48) as u16,
            (r >> 32) as u16,
        )
    }

    pub fn xy(&self) -> (u16, u16) {
        let r = self.0;
        ((r >> 16) as u16, r as u16)
    }

    pub fn wh(&self) -> (u16, u16) {
        let r = self.0;
        ((r >> 48) as u16, (r >> 32) as u16)
    }

    pub fn packed_whxy(&self) -> u64 {
        self.0
    }

    pub fn area(&self) -> u32 {
        let (w, h) = self.wh();
        w as u32 * h as u32
    }
}

pub trait Packer {
    fn new(width: u16, height: u16) -> Self;
    fn pack(&mut self, w: u16, h: u16) -> Option<(u16, u16)>;
    fn pack_all(&mut self, rects: &[(u16, u16)]) -> Vec<Option<(u16, u16)>>;
    #[allow(unused)]
    fn unpack(&mut self, x: u16, y: u16, w: u16, h: u16) {
        panic!("unpacking not supported for this packer")
    }
    #[allow(unused)]
    fn reset(&mut self);
    fn resize(&mut self, width: u16, height: u16);
    fn width(&self) -> u16;
    fn height(&self) -> u16;
}

pub struct Guillotine {
    width: u16,
    height: u16,
    pub free_rects: Vec<Rect>, // FIXME: remove pub, it was for testing
}

enum AdjRect {
    North(usize),
    East(usize),
    South(usize),
    West(usize),
    None,
}

impl Guillotine {
    fn split(fw: u16, fh: u16, x: u16, y: u16, w: u16, h: u16) -> (Rect, Rect) {
        // more free space on top
        if fh - h > fw - w {
            (
                Rect::new(x, y + h, fw, fh - h), // wide top
                Rect::new(x + w, y, fw - w, h),  // narrow right
            )
        } else {
            (
                Rect::new(x + w, y, fw - w, fh), // tall right
                Rect::new(x, y + h, w, fh - h),  // short top
            )
        }
    }

    /// returns indices of north/east/south/west adjacent rects with same edge length
    fn find_adjacent(&self, x: u16, y: u16, w: u16, h: u16) -> [Option<usize>; 4] {
        let (mut north, mut east, mut south, mut west) = (None, None, None, None);
        for (i, fr) in self.free_rects.iter().enumerate() {
            let (fx, fy, fw, fh) = fr.xywh();
            if fx == x && fy == y + h && fw == w {
                north = Some(i);
            } else if fx == x + w && fy == y && fh == h {
                east = Some(i);
            } else if fx == x && fy + fh == y && fw == w {
                south = Some(i);
            } else if fx + fw == x && fy == y && fh == h {
                west = Some(i);
            }
            if north.is_some() && east.is_some() && south.is_some() && west.is_some() {
                break;
            }
        }
        [north, east, south, west]
    }

    fn find_biggest_adjacent(&self, x: u16, y: u16, w: u16, h: u16) -> AdjRect {
        let (i, adj_idx) = self
            .find_adjacent(x, y, w, h)
            .into_iter()
            .enumerate()
            .max_by_key(|(_, idx)| idx.map(|i| self.free_rects[i].area()).unwrap_or(0))
            .unwrap();
        if let Some(adj_idx) = adj_idx {
            match i {
                0 => AdjRect::North(adj_idx),
                1 => AdjRect::East(adj_idx),
                2 => AdjRect::South(adj_idx),
                3 => AdjRect::West(adj_idx),
                _ => unreachable!(),
            }
        } else {
            AdjRect::None
        }
    }

    /// finds adjacent rects and merges with biggest one `(by area)`\
    /// keeps doing this until everything is fully merged `(greedy)`
    // can also consider adjacent rects with different edge length
    // and merge them if merging doesn't leave high aspect ratio rects
    // then merge the left out rect (since it's dimensions changed)
    fn merge(&mut self, free_rect_idx: usize) {
        let (x, y, w, h) = self.free_rects[free_rect_idx].xywh();
        assert_ne!(w, 0, "width was 0");
        assert_ne!(h, 0, "height was 0");
        let adj = self.find_biggest_adjacent(x, y, w, h);
        let new_rect = match adj {
            AdjRect::North(i) => {
                let (_, fh) = self.free_rects[i].wh();
                Some((i, Rect::new(x, y, w, h + fh)))
            }
            AdjRect::East(i) => {
                let (fw, _) = self.free_rects[i].wh();
                Some((i, Rect::new(x, y, w + fw, h)))
            }
            AdjRect::South(i) => {
                let (_, fh) = self.free_rects[i].wh();
                Some((i, Rect::new(x, y - fh, w, fh + h)))
            }
            AdjRect::West(i) => {
                let (fw, _) = self.free_rects[i].wh();
                Some((i, Rect::new(x - fw, y, fw + w, h)))
            }
            AdjRect::None => None,
        };
        if let Some((i, nr)) = new_rect {
            self.free_rects.swap_remove(free_rect_idx);
            if i == self.free_rects.len() {
                self.free_rects.swap_remove(free_rect_idx);
            } else {
                self.free_rects.swap_remove(i);
            }
            self.free_rects.push(nr);
            self.merge(self.free_rects.len() - 1);
        }
    }
}

impl Packer for Guillotine {
    fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            free_rects: vec![Rect::new(0, 0, width, height)],
        }
    }

    /// O(n * r), n is free rect count, r is rects.len()
    fn pack_all(&mut self, rects: &[(u16, u16)]) -> Vec<Option<(u16, u16)>> {
        let mut indexed_rects = rects
            .iter()
            .enumerate()
            .map(|(i, &(w, h))| (i, w, h))
            .collect::<Vec<_>>();
        indexed_rects.sort_unstable_by_key(|&(_, w, h)| std::cmp::Reverse(w as u32 * h as u32));
        let mut rects = vec![None; indexed_rects.len()];
        for (i, w, h) in indexed_rects {
            let r = self.pack(w, h);
            rects[i] = r;
        }
        rects
    }

    /// O(n), n is free rect count
    fn pack(&mut self, w: u16, h: u16) -> Option<(u16, u16)> {
        assert_ne!(w, 0, "width was 0");
        assert_ne!(h, 0, "height was 0");
        let min_fr = self.free_rects.iter_mut().enumerate().min_by_key(|(_, f)| {
            let (fw, fh) = f.wh();
            if fw < w || fh < h {
                u32::MAX
            } else {
                ((fw - w) as u32 * h as u32).min((fh - h) as u32 * w as u32)
            }
        });
        if let Some((i, min_fr)) = min_fr {
            let (x, y, fw, fh) = min_fr.xywh();
            if w <= fw && h <= fh {
                use std::cmp::Ordering::*;
                match (w.cmp(&fw), h.cmp(&fh)) {
                    (_, Greater) | (Greater, _) => unreachable!(),
                    (Less, Less) => {
                        let (big, small) = Self::split(fw, fh, x, y, w, h);
                        *min_fr = small;
                        self.free_rects.push(big);
                    }
                    (Less, Equal) => {
                        *min_fr = Rect::new(x + w, y, fw - w, fh);
                    }
                    (Equal, Less) => {
                        *min_fr = Rect::new(x, y + h, fw, fh - h);
                    }
                    (Equal, Equal) => {
                        self.free_rects.swap_remove(i);
                    }
                };
                Some((x, y))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn unpack(&mut self, x: u16, y: u16, w: u16, h: u16) {
        self.free_rects.push(Rect::new(x, y, w, h));
        self.merge(self.free_rects.len() - 1);
    }

    fn reset(&mut self) {
        self.free_rects = vec![Rect::new(0, 0, self.width, self.height)];
    }

    fn resize(&mut self, width: u16, height: u16) {
        let (fw, fh) = (width, height);
        let (w, h) = (self.width, self.height);
        assert!(fw >= w && fh >= h);
        if fw == w && fh == h {
            return;
        }
        self.width = fw;
        self.height = fh;
        let (big, small) = Self::split(fw, fh, 0, 0, w, h);
        self.free_rects.push(small);
        self.merge(self.free_rects.len() - 2);
        self.free_rects.push(big);
        self.merge(self.free_rects.len() - 1);
    }

    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }
}

// TODO: skyline/shelf algos
