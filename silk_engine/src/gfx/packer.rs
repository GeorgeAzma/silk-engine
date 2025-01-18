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

    pub fn packed_whxy(&self) -> u64 {
        self.0
    }
}

pub struct Packer {
    width: u16,
    height: u16,
    free_rects: Vec<Rect>,
}

impl Packer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            free_rects: vec![Rect::new(0, 0, width, height)],
        }
    }

    pub fn pack(&mut self, w: u16, h: u16) -> Option<(u16, u16)> {
        for i in 0..self.free_rects.len() {
            let (x, y, fw, fh) = self.free_rects[i].xywh();
            if w <= fw && h <= fh {
                return if w < fw && h < fh {
                    if fw - w < fh - h {
                        self.free_rects[i] = Rect::new(x + w, y, fw - w, h);
                        self.free_rects.push(Rect::new(x, y + h, fw, fh - h));
                    } else {
                        self.free_rects[i] = Rect::new(x, y + h, w, fh - h);
                        self.free_rects.push(Rect::new(x + w, y, fw - w, fh));
                    }
                    Some((x, y))
                } else if h == fh {
                    self.free_rects[i] = Rect::new(x + w, y, fw - w, fh);
                    Some((x, y))
                } else if w == fw {
                    self.free_rects[i] = Rect::new(x, y + h, fw, fh - h);
                    Some((x, y))
                } else {
                    self.free_rects.swap_remove(i);
                    Some((x, y))
                };
            }
        }
        None
    }

    #[allow(unused)]
    pub fn reset(&mut self) {
        self.free_rects = vec![Rect::new(0, 0, self.width, self.height)];
    }

    #[allow(unused)]
    pub fn resize(&mut self, width: u16, height: u16) {
        assert!(width >= self.width && height >= self.height);
        if width == self.width && height == self.height {
            return;
        }
        let (big, small) = if width - self.width < height - self.height {
            (
                Rect::new(0, self.height, width, height - self.height),
                Rect::new(self.width, 0, width - self.width, self.height),
            )
        } else {
            (
                Rect::new(self.width, 0, width - self.width, height),
                Rect::new(0, self.height, self.width, height - self.height),
            )
        };
        self.free_rects.push(big);
        self.free_rects.push(small);
        self.width = width;
        self.height = height;
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }
}
