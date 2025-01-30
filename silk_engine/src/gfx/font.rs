use std::collections::HashMap;

use super::{
    RenderCtx,
    packer::{Guillotine, Packer, Rect},
};
use crate::util::{Bmp, ExtraFns, ImageFormat, Ttf, Vec2, Vec2u, Vec3, Vectorf};

// https://www.shadertoy.com/view/ftdGDB
fn bezier_sdf(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> f32 {
    const EPS: f32 = 1e-6;
    let aa = b - a;
    let bb = a - 2.0 * b + c;
    let cc = aa * 2.0;
    let d = a - p;

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

pub struct Font;

impl Font {
    pub fn new(name: &str, char_size_px: u32) -> Self {
        let t = crate::util::print::ScopeTime::new(&format!("parse font({name})"));
        let mut reader = Ttf::new(name);
        // extract ascii glyphs
        reader.head.glob_xmin = i16::MAX;
        reader.head.glob_ymin = i16::MAX;
        reader.head.glob_xmax = i16::MIN;
        reader.head.glob_ymax = i16::MIN;
        let mut glyphs = Vec::with_capacity(128);
        let uni2idx: HashMap<char, u32> = reader
            .idx2uni
            .iter()
            .enumerate()
            .map(|(i, uni)| (*uni, i as u32))
            .collect();
        for ascii in (0u8..128)
            .map(|x| x as char)
            .filter(|x| x.is_ascii_graphic())
        {
            let idx = uni2idx[&ascii] as usize;
            let glyph = &reader.glyphs[idx];
            let (w, h) = (glyph.metric.width(), glyph.metric.height());
            if w == 0 || h == 0 {
                continue;
            }
            glyphs.push(glyph.clone());
            reader.head.glob_xmin = reader.head.glob_xmin.min(glyph.metric.xmin);
            reader.head.glob_ymin = reader.head.glob_ymin.min(glyph.metric.ymin);
            reader.head.glob_xmax = reader.head.glob_xmax.max(glyph.metric.xmax);
            reader.head.glob_ymax = reader.head.glob_ymax.max(glyph.metric.ymax);
        }
        reader.head.num_glyphs = glyphs.len() as u16;

        let num_glyphs = reader.head.num_glyphs;
        let (mx, my) = (reader.head.max_width(), reader.head.max_height());
        let (nx, ny) = (1.0 / mx as f32, 1.0 / my as f32);
        let padding_px: u32 = char_size_px / 16 + 4;

        let mut unpacked = Vec::with_capacity(num_glyphs as usize);
        let mut area_px = 0;
        for glyph in glyphs.iter() {
            let (w, h) = (glyph.metric.width(), glyph.metric.height());
            let (w, h) = (w as f32 * nx, h as f32 * ny);
            assert!(w <= 1.0 && h <= 1.0, "invalid glyph width/height: {w}x{h}");
            let (w, h) = (
                (w * char_size_px as f32).ceil() as u16 + padding_px as u16,
                (h * char_size_px as f32).ceil() as u16 + padding_px as u16,
            );
            unpacked.push((w, h));
            area_px += w as u32 * h as u32;
        }
        // NOTE: might need to be multiple of 256 for vulkan image transfer alignment requirements
        //       (also would match work group size in font sdf shader)

        let mut font_sdf_dim = (((area_px as f32).sqrt()) as u32).next_multiple_of(4);

        // write font bezier points into buffer
        let mut font_points = vec![];
        let mut off_sizes = Vec::with_capacity(num_glyphs as usize);
        let pad = padding_px as f32 / char_size_px as f32 * 0.5;
        for glyph in glyphs.iter() {
            let mut csi = 0;
            let off = font_points.len() as u32;
            for &cei in glyph.contour_end_idxs.iter() {
                let mut points = Self::convert_points(
                    &glyph.points[csi..cei as usize + 1],
                    glyph.metric.xmin - (pad * mx as f32).round() as i16,
                    glyph.metric.ymin - (pad * my as f32).round() as i16,
                    mx,
                    my,
                );
                assert!(points.len() >= 3, "must have atleast 3 points for bezier");
                font_points.append(&mut points);
                csi = cei as usize + 1;
            }
            let size = (font_points.len() as u32 - off) / 3;
            off_sizes.push((off, size));
        }
        let mut packer = Guillotine::new(font_sdf_dim as u16, font_sdf_dim as u16);
        let packed = packer.growing_pack_all_with(&unpacked, |w: u16, h: u16| {
            (
                ((w as f32 * 1.02).ceil() as u16).next_multiple_of(4),
                ((h as f32 * 1.02).ceil() as u16).next_multiple_of(4),
            )
        });
        font_sdf_dim = packer.width() as u32;
        let mut font_glyphs = vec![[0u32; 4]; num_glyphs as usize];
        for (i, &(x, y)) in packed.iter().enumerate() {
            let (w, h) = unpacked[i];
            let r = Rect::new(x, y, w, h).packed_whxy();
            let wh = (r >> 32) as u32;
            let xy = r as u32;
            let (off, size) = off_sizes[i];
            let glyph = [off, size, wh, xy];
            font_glyphs[i] = glyph;
        }
        drop(t);

        let font_sdf_pxs = font_sdf_dim * font_sdf_dim;
        let t = crate::util::print::ScopeTime::new(&format!("{name} sdf gen"));
        let mut font_sdf = vec![0u8; font_sdf_pxs as usize];
        for [off, size, wh, xy] in font_glyphs {
            let gs = Vec2u::new(wh >> 16, wh & 0xFFFF);
            let gp = Vec2u::new(xy >> 16, xy & 0xFFFF);
            for y in 0..gs.y {
                for x in 0..gs.x {
                    let pu = Vec2u::new(x + gp.x, y + gp.y);
                    let p = Vec2::from(pu - gp) / Vec2::from(char_size_px);
                    let mut d = f32::MAX;
                    for i in 0..size {
                        let off = off as usize + i as usize * 3;
                        let a = Vec2::from(font_points[off + 0]);
                        let b = Vec2::from(font_points[off + 1]);
                        let c = Vec2::from(font_points[off + 2]);
                        let (min, max) = (a.min(b).min(c) - 0.1, a.max(b).max(c) + 0.1);
                        if p.x < min.x || p.y < min.y || p.x > max.x || p.y > max.y {
                            continue;
                        }
                        let dir = (c - a).norm() * 5e-5;
                        let bd = bezier_sdf(p, a + dir, b + dir, c - dir);
                        if bd.abs() < d.abs() {
                            d = bd;
                        }
                    }
                    let d = d * 4.0 + 0.75;
                    if d <= 1.0 {
                        font_sdf[(pu.y * font_sdf_dim + pu.x) as usize] |=
                            (d.saturate() * 255.0) as u8;
                    }
                }
            }
        }

        drop(t);
        Bmp::save("temp", &font_sdf[..], font_sdf_dim, font_sdf_dim, 1);

        Self
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
}
