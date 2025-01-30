use std::collections::HashMap;

use super::{
    BufUsage, ImageFormat, MemProp, RenderCtx,
    font_reader::FontReader,
    packer::{Guillotine, Packer, Rect},
};
use crate::bmp::Bmp;

// fn cross2(a: vec2f, b: vec2f) -> f32 {
//     return a.x * b.y - a.y * b.x;
// }

// // https://www.shadertoy.com/view/ftdGDB
// fn bezier_sdf(p: vec2f, A: vec2f, B: vec2f, C: vec2f) -> f32 {
//     let EPS = 1e-6;
//     let a = B - A;
//     let b = A - 2.0 * B + C;
//     let c = a * 2.0;
//     let d = A - p;

//     let kk = 1.0 / dot(b, b);
//     let kx = kk * dot(a, b);
//     let ky = kk * (2.0 * dot(a, a) + dot(d, b)) / 3.0;
//     let kz = kk * dot(d, a);

//     let mut res = 0.0;
//     let mut sgn = 0.0;

//     let p1 = ky - kx * kx;
//     let p3 = p1 * p1 * p1;
//     let q = kx * (2.0 * kx * kx - 3.0 * ky) + kz;
//     let mut h = q * q + 4.0 * p3;
//     if h >= 0.0 {
//         h = h.sqrt();
//         let x = 0.5 * (vec2f(h, -h) - q);
//         let uv = sign(x) * pow(abs(x), vec2f(1.0 / 3.0));
//         let t = saturate(uv.x + uv.y - kx) + EPS;
//         let q = d + (c + b * t) * t;
//         res = dot(q, q);
//         sgn = cross2(c + 2.0 * b * t, q);
//     } else {
//         let z = sqrt(-p1);
//         let v = acos(q / (p1 * z * 2.0)) / 3.0;
//         let m = cos(v);
//         let n = sin(v) * sqrt(3.0);
//         let t = saturate(vec3f(m + m, -n - m, n - m) * z - kx) + EPS;
//         let qx = d + (c + b * t.x) * t.x;
//         let dx = dot(qx, qx);
//         let sx = cross2(c + 2.0 * b * t.x, qx);
//         let qy = d + (c + b * t.y) * t.y;
//         let dy = dot(qy, qy);
//         let sy = cross2(c + 2.0 * b * t.y, qy);
//         res = select(dy, dx, dx < dy);
//         sgn = select(sy, sx, dx < dy);
//     }
//     return sign(sgn) * sqrt(res);
// }

pub struct Font;

impl Font {
    pub fn new(name: &str, char_size_px: u32, ctx: &mut RenderCtx) -> Self {
        let t = crate::ScopeTime::new(&format!("parse font({name})"));
        let mut reader = FontReader::new(name);
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

        ctx.add_compute("font-sdf");
        ctx.add_desc_set("font sdf ds", "font-sdf", 0);
        ctx.add_buf(
            "font sdf",
            font_sdf_pxs as u64,
            BufUsage::SRC | BufUsage::STORAGE,
            MemProp::GPU,
        );
        ctx.add_buf(
            "font sdf uniform",
            size_of::<[u32; 2]>() as u64,
            BufUsage::UNIFORM,
            MemProp::CPU_GPU,
        );
        ctx.add_buf(
            "font points",
            (font_points.len() * size_of::<[f32; 2]>()) as u64,
            BufUsage::STORAGE,
            MemProp::CPU_GPU,
        );
        ctx.add_buf(
            "font glyphs",
            num_glyphs as u64 * size_of::<[u32; 4]>() as u64,
            BufUsage::STORAGE,
            MemProp::CPU_GPU,
        );
        // TODO: only write if necessary (at init and when buffer resized)
        ctx.write_ds_bufs("font sdf ds", &[
            ("font sdf", 0),
            ("font sdf uniform", 1),
            ("font points", 2),
            ("font glyphs", 3),
        ]);

        let t = crate::ScopeTime::new(&format!("{name} sdf gen"));
        ctx.write_buf("font sdf uniform", &[font_sdf_dim, char_size_px]);
        ctx.write_buf("font points", &font_points[..]);
        ctx.write_buf("font glyphs", &font_glyphs[..]);

        ctx.begin_cmd();
        ctx.bind_pipeline("font-sdf");
        ctx.bind_ds("font sdf ds");
        ctx.dispatch(font_sdf_pxs / 4, num_glyphs as u32, 1);
        ctx.finish_cmd();

        let mut font_sdf = vec![0u8; font_sdf_pxs as usize];
        ctx.read_buf("font sdf", &mut font_sdf[..]);
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
