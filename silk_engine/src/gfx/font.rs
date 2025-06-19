use std::collections::HashMap;

use crate::util::{ExtraFns, GlyphData, ImageData, Ttf, Vec2, Vec2u, Vectorf};

pub struct Font {
    uni2glyph: HashMap<char, GlyphData>,
    kernings: HashMap<u32, i16>,
    ascent: i16,
    descent: i16,
    line_gap: i16,
    max_width: u16,  // max glyph width in em
    max_height: u16, // max glyph height in em
    em_units: u16,
}

impl Font {
    pub fn new(name: &str) -> Self {
        let ttf = Ttf::new(name);
        let uni2glyph: HashMap<char, GlyphData> = ttf
            .idx2uni
            .iter()
            .enumerate()
            .map(|(i, uni)| (*uni, ttf.glyphs[i].clone()))
            .collect();
        Self {
            uni2glyph,
            kernings: ttf.kernings,
            ascent: ttf.ascent,
            descent: ttf.descent,
            line_gap: ttf.line_gap,
            max_width: ttf.head.max_width(),
            max_height: ttf.head.max_height(),
            em_units: ttf.head.em_units,
        }
    }

    fn em(&self, em_unit: i16) -> f32 {
        em_unit as f32 / self.em_units as f32
    }

    /// returns `str` layout, where 1.0 is `em_units`
    /// accounts for `[' ', '\n', '\r', '\t']`
    pub fn layout(&self, text: &str) -> Vec<(f32, f32)> {
        let (mut x, mut y) = (0.0, 0.0);
        let mut positions = Vec::with_capacity(text.len());
        let mut prev_c = '\0';
        let space_width = self
            .uni2glyph
            .get(&' ')
            .map(|g| self.em(g.metric.advance_width))
            .unwrap_or(1.0);
        let line_height: f32 = self.em(self.ascent - self.descent + self.line_gap);
        let gap: f32 = 0.15;
        for c in text.chars() {
            let mut pos = (0.0, 0.0);
            match c {
                ' ' => {
                    x += space_width + gap;
                }
                '\n' => {
                    x = 0.0;
                    y -= line_height;
                }
                '\r' => {
                    x = 0.0;
                }
                '\t' => {
                    x += space_width * 4.0;
                }
                _ => {
                    if let Some(glyph) = self.uni2glyph.get(&c) {
                        let is_cjk = self.is_char_cjk(c);
                        if !is_cjk {
                            if let Some(prev_glyph) = self.uni2glyph.get(&prev_c) {
                                let kerning = self.kerning(prev_glyph, glyph);
                                x += self.em(kerning);
                            }
                        }
                        let m = &glyph.metric;
                        let x_off = self.em(m.xmin);
                        let y_off = self.em(m.ymin);
                        pos = (x + x_off, y + y_off);
                        if is_cjk {
                            x += self.em(m.width()) + gap
                        }
                        x += self.em(m.advance_width) + gap;
                        prev_c = c;
                    }
                }
            }
            positions.push(pos);
        }
        positions
    }

    pub fn bounding_box(&self, str: &str) -> (f32, f32, f32, f32) {
        let layout = self.layout(str);
        let (mut ax, mut ay, mut bx, mut by) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (c, (lx, ly)) in str.chars().zip(layout.into_iter()) {
            ax = ax.min(lx);
            ay = ay.min(ly);
            let (w, h) = self.glyph_size(c);
            bx = bx.max(lx + w);
            by = by.max(ly + h);
        }
        (ax, ay, bx, by)
    }

    pub fn bounding_rect(&self, str: &str) -> (f32, f32, f32, f32) {
        let (ax, ay, bx, by) = self.bounding_box(str);
        (ax, ay, bx - ax, by - ay)
    }

    fn kerning(&self, a: &GlyphData, b: &GlyphData) -> i16 {
        *self
            .kernings
            .get(&(((a.index as u32) << 16) | b.index as u32))
            .unwrap_or(&0)
    }

    pub fn glyph_size(&self, char: char) -> (f32, f32) {
        let Some(glyph) = &self.uni2glyph.get(&char) else {
            return (0.0, 0.0);
        };
        (
            self.em(glyph.metric.width()),
            self.em(glyph.metric.height()),
        )
    }

    pub fn is_char_graphic(&self, char: char) -> bool {
        self.uni2glyph
            .get(&char).is_some_and(|g| g.points.len() > 1)
    }

    pub fn is_char_cjk(&self, char: char) -> bool {
        matches!(char,
            '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
            '\u{3040}'..='\u{309F}' |  // Hiragana
            '\u{30A0}'..='\u{30FF}' |  // Katakana
            '\u{FF00}'..='\u{FF9F}'    // Full-width Roman characters and half-width Katakana
        )
    }

    pub fn gen_char_sdf(&self, char: char, size_px: u32) -> ImageData {
        crate::scope_time!("gen '{char}' sdf {size_px}px");

        if !self.is_char_graphic(char) {
            return ImageData::new(vec![], 0, 0, 0);
        }
        let (mx, my) = (self.max_width, self.max_height);
        let (mx, my) = (mx.max(my), mx.max(my));
        let (nx, ny) = (1.0 / mx as f32, 1.0 / my as f32);
        let padding_px: u32 = size_px.div_ceil(8);
        let glyph = &self.uni2glyph[&char];
        let (ew, eh) = (glyph.metric.width(), glyph.metric.height());
        let (nw, nh) = (ew as f32 * nx, eh as f32 * ny);
        assert!(nw <= 1.0 && nh <= 1.0);
        let (w, h) = (
            (nw * size_px as f32).ceil() as u16 + padding_px as u16,
            (nh * size_px as f32).ceil() as u16 + padding_px as u16,
        );
        let pad = padding_px as f32 / size_px as f32 * 0.5;
        let mut csi = 0;
        let mut points = vec![];
        for &cei in glyph.contour_end_idxs.iter() {
            let mut contour_points = convert_points(
                &glyph.points[csi..cei as usize + 1],
                glyph.metric.xmin - (pad * mx as f32).round() as i16,
                glyph.metric.ymin - (pad * my as f32).round() as i16,
                mx,
                my,
            );
            points.append(&mut contour_points);
            csi = cei as usize + 1;
        }

        fn convert_points(
            points: &[(i16, i16, bool)],
            xmin: i16,
            ymin: i16,
            w: u16,
            h: u16,
        ) -> Vec<(f32, f32)> {
            let mut new_points = Vec::with_capacity(points.len() * 2);
            let norm_x = |x: i16| (x - xmin) as f32 / w as f32;
            let norm_y = |y: i16| (y - ymin) as f32 / h as f32;
            let on_curve_off = points.iter().position(|(_, _, c)| *c).unwrap();
            for i0 in 0..points.len() {
                let i0 = (i0 + on_curve_off/**/) % points.len();
                let i1 = (i0 + on_curve_off + 1) % points.len();
                let (x0, y0, c0) = points[i0];
                let (x1, y1, c1) = points[i1];
                let (x0, y0) = (norm_x(x0), norm_y(y0));
                let (x1, y1) = (norm_x(x1), norm_y(y1));
                new_points.push((x0, y0));
                // insert midpoint between 2 on/off-curve points
                if c0 == c1 {
                    let mx = (x0 + x1) * 0.5;
                    let my = (y0 + y1) * 0.5;
                    new_points.push((mx, my));
                }
            }

            let mut duped_points = Vec::with_capacity(new_points.len() * 2);
            for i in (0..new_points.len()).step_by(2) {
                duped_points.push(new_points[i]);
                duped_points.push(new_points[(i + 1) % new_points.len()]);
                duped_points.push(new_points[(i + 2) % new_points.len()]);
            }
            duped_points
        }

        struct Segment {
            a: Vec2,
            b: Vec2,
            c: Vec2,
            min: Vec2,
            max: Vec2,
            line: bool,
        }

        impl Segment {
            // https://www.shadertoy.com/view/ftdGDB
            fn sdf(&self, p: Vec2) -> f32 {
                if self.line {
                    let ba = self.c - self.a;
                    let pa = p - self.a;
                    let h = pa.dot(ba) / ba.len2();
                    return pa.cross(ba).signum() * (ba * h.saturate() - pa).len();
                }
                let ab = self.b - self.a;
                let d = self.c - 2.0 * self.b + self.a;
                let mut t = ((p - self.a).dot(ab) / ab.len2()).saturate();
                // Newton iters
                for _ in 0..2 {
                    let q = self.a + ab * (2.0 * t) + d * (t * t);
                    let dq = ab * 2.0 + d * (2.0 * t);
                    let f = (q - p).dot(dq);
                    let df = dq.len2() + (q - p).dot(2.0 * d);
                    let dt = -f / df;
                    t = (t + dt).saturate();
                }
                let q = self.a + ab * (2.0 * t) + d * (t * t);
                let dist = (q - p).len();
                let dq = ab * 2.0 + d * (2.0 * t);
                let sign = dq.cross(q - p).signum();
                sign * dist
            }
        }

        struct SpatialGrid {
            cells: Vec<Vec<usize>>,
        }

        impl SpatialGrid {
            const CELLS: u32 = 16;
            fn new(segments: &[Segment]) -> Self {
                let mut cells = vec![vec![]; (Self::CELLS * Self::CELLS) as usize];
                for (i, s) in segments.iter().enumerate() {
                    let min: Vec2u = (s.min * Self::CELLS as f32).floor().into();
                    let max: Vec2u = (s.max * Self::CELLS as f32).ceil().into();
                    for y in min.y..=max.y {
                        for x in min.x..=max.x {
                            if x < Self::CELLS && y < Self::CELLS {
                                let idx = y * Self::CELLS + x;
                                cells[idx as usize].push(i);
                            }
                        }
                    }
                }
                Self { cells }
            }

            fn get(&self, p: Vec2) -> &[usize] {
                let x = (p.x * Self::CELLS as f32).floor() as u32;
                let y = (p.y * Self::CELLS as f32).floor() as u32;
                &self.cells[(y * Self::CELLS + x) as usize]
            }
        }

        let mut segments = Vec::with_capacity(points.len() / 3);
        for i in 0..points.len() / 3 {
            let idx = i * 3;
            let a = Vec2::from(points[idx]);
            let b = Vec2::from(points[idx + 1]);
            let c = Vec2::from(points[idx + 2]);
            let dir = (c - a).norm() * 5e-5;
            let (a, b, c) = (a + dir, b + dir, c - dir);
            let (min, max) = (a.min(b).min(c) - 0.04, a.max(b).max(c) + 0.04);
            let curve = (b - (a + c) * 0.5).len2() / (c - a).len2();
            segments.push(Segment {
                a,
                b,
                c,
                min,
                max,
                line: curve < 0.01,
            });
        }
        segments.sort_unstable_by(|a, b| a.min.y.total_cmp(&b.min.y));
        let grid = SpatialGrid::new(&segments);

        let mut sdf = vec![0; w as usize * h as usize];
        let rcp_size = Vec2::from(1.0 / size_px as f32);
        for y in 0..h {
            for x in 0..w {
                let p = Vec2::new(x as f32, y as f32) * rcp_size;
                let mut d = f32::MAX;
                let segs = grid.get(p);
                if segs.is_empty() {
                    continue;
                }
                for &i in segs {
                    let s = &segments[i];
                    if p.y < s.min.y {
                        break;
                    }
                    if p.x < s.min.x || p.x > s.max.x || p.y > s.max.y {
                        continue;
                    }
                    // barycentric triangle intersection (useful for > 64px)
                    // let c1 = (self.b - self.a).cross(p - self.a);
                    // let c2 = (self.c - self.b).cross(p - self.b);
                    // let c3 = (self.a - self.c).cross(p - self.c);
                    // if !(c1 >= -0.02 && c2 >= -0.02 && c3 >= -0.02) {
                    //     return 1e9;
                    // }
                    let bd = s.sdf(p);
                    if bd.abs() < d.abs() {
                        d = bd;
                    }
                }
                const E: f32 = 0.0625;
                let d = (d + E) / (2.0 * E);
                if d <= 1.0 {
                    sdf[(y * w + x) as usize] |= (d.saturate() * 255.0) as u8;
                }
            }
        }
        ImageData::new(sdf, w as u32, h as u32, 1)
    }
}
