use std::collections::HashMap;

use crate::util::{ExtraFns, GlyphData, ImageData, Ttf, Vec2, Vec2u, Vec3, Vectorf};

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
            .get(&((a.index as u32) << 16 | b.index as u32))
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
            .get(&char)
            .map_or(false, |g| g.points.len() > 1)
    }

    pub fn is_char_cjk(&self, char: char) -> bool {
        matches!(char,
            '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
            '\u{3040}'..='\u{309F}' |  // Hiragana
            '\u{30A0}'..='\u{30FF}' |  // Katakana
            '\u{FF00}'..='\u{FF9F}'    // Full-width Roman characters and half-width Katakana
        )
    }

    // TODO: speedup sdf generation, takes 5ms for 64x64 chinese char gen
    pub fn gen_char_sdf(&self, char: char, size_px: u32) -> ImageData {
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
        }

        impl Segment {
            // https://www.shadertoy.com/view/ftdGDB
            fn sdf(&self, p: Vec2) -> f32 {
                const EPS: f32 = 1e-6;
                let aa = self.b - self.a;
                let bb = self.a - 2.0 * self.b + self.c;
                let cc = aa * 2.0;
                let d = self.a - p;

                let kk = 1.0 / bb.len2();
                let kx = kk * aa.dot(bb);
                let ky = kk * (2.0 * aa.len2() + d.dot(bb)) / 3.0;
                let kz = kk * d.dot(aa);

                let res;
                let sgn;
                let p1 = ky - kx * kx;
                let p3 = p1 * p1 * p1;
                let q = kx * (2.0 * kx * kx - 3.0 * ky) + kz;
                let mut h = q * q + 4.0 * p3;
                if h >= 0.0 {
                    h = h.sqrt();
                    let x = 0.5 * (Vec2::new(h, -h) - q);
                    let uv = x.sign() * x.abs().cbrt();
                    let t = (uv.x + uv.y - kx).saturate() + EPS;
                    let q = d + (cc + bb * t) * t;
                    res = q.len2();
                    sgn = (cc + 2.0 * bb * t).cross(q);
                } else {
                    let z = (-p1).sqrt();
                    let v = (q / (p1 * z * 2.0)).acos() / 3.0;
                    let m = v.cos();
                    let n = v.sin() * 3f32.sqrt();
                    let t = (Vec3::new(m + m, -n - m, n - m) * z - kx).saturate() + EPS;
                    let qx = d + (cc + bb * t.x) * t.x;
                    let dx = qx.len2();
                    let sx = (cc + 2.0 * bb * t.x).cross(qx);
                    let qy = d + (cc + bb * t.y) * t.y;
                    let dy = qy.len2();
                    let sy = (cc + 2.0 * bb * t.y).cross(qy);
                    res = dx.min(dy);
                    sgn = if dx < dy { sx } else { sy };
                }
                sgn.signum() * res.sqrt()
            }
        }

        struct SpatialGrid {
            cells: Vec<Vec<usize>>,
        }

        impl SpatialGrid {
            const CELLS: u32 = 8;
            fn new(segments: &[Segment]) -> Self {
                let mut cells = vec![vec![]; (Self::CELLS * Self::CELLS) as usize];
                for (i, s) in segments.iter().enumerate() {
                    let min: Vec2u = (s.min * Self::CELLS as f32).floor().into();
                    let max: Vec2u = (s.max * Self::CELLS as f32).ceil().into();
                    for y in min.y..=max.y {
                        for x in min.x..=max.x {
                            if x < Self::CELLS && y < Self::CELLS {
                                let idx = y * Self::CELLS + x;
                                cells[idx as usize].push(i as usize);
                            }
                        }
                    }
                }
                Self { cells }
            }

            fn get(&self, p: Vec2) -> &[usize] {
                let x = (p.x * Self::CELLS as f32).floor() as u32;
                let y = (p.y * Self::CELLS as f32).floor() as u32;
                if x < Self::CELLS && y < Self::CELLS {
                    &self.cells[(y * Self::CELLS + x) as usize]
                } else {
                    &[]
                }
            }
        }

        let mut segments = Vec::with_capacity(points.len() / 3);
        for i in 0..points.len() / 3 {
            let idx = i as usize * 3;
            let a = Vec2::from(points[idx + 0]);
            let b = Vec2::from(points[idx + 1]);
            let c = Vec2::from(points[idx + 2]);
            let dir = (c - a).norm() * 5e-5;
            let (a, b, c) = (a + dir, b + dir, c - dir);
            let (min, max) = (a.min(b).min(c) - 0.05, a.max(b).max(c) + 0.05);
            segments.push(Segment { a, b, c, min, max });
        }
        let grid = SpatialGrid::new(&segments);

        let mut sdf = vec![0; w as usize * h as usize];
        for y in 0..h {
            for x in 0..w {
                let p = Vec2::new(x as f32, y as f32) / Vec2::from(size_px);
                let mut d = f32::MAX;
                for &i in grid.get(p) {
                    let s = &segments[i];
                    if p.x < s.min.x || p.x > s.max.x || p.y < s.min.y || p.y > s.max.y {
                        continue;
                    }
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
