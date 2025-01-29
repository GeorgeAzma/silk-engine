use crate::RES_PATH;

pub struct ImageData {
    pub img: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub channels: u8,
}

impl ImageData {
    pub fn new(img: Vec<u8>, width: u32, height: u32, channels: u8) -> Self {
        Self {
            img,
            width,
            height,
            channels,
        }
    }
}

const MAX_PIXELS: u32 = 400_000_000;
const SRGB: u8 = 0;
const LINEAR: u8 = 1;
const RGB_MASK: u8 = 0b1111_1110;
const RGBA_MASK: u8 = 0b1111_1111;
const INDEX_MASK: u8 = 0b0000_0000;
const DIFF_MASK: u8 = 0b0100_0000;
const LUMA_MASK: u8 = 0b1000_0000;
const RUN_MASK: u8 = 0b1100_0000;
const MIN_QOI_LEN: usize = 4 /* magic */ + 4 /* width */ + 4 /* height */ + 1 /* channels */ + 1 /* colorspace */ + 8 /* padding */;

pub struct Qoi;
impl Qoi {
    pub fn load(name: &str) -> ImageData {
        crate::scope_time!("QOI load");
        let path = format!("{RES_PATH}/images/{name}.qoi");
        let qoi = std::fs::read(path).unwrap_or_else(|_| panic!("qoi image not found: {name}"));
        assert_eq!(
            &qoi[0..4],
            b"qoif",
            "invalid qoi magic number: {}",
            std::str::from_utf8(&qoi[0..4])
                .unwrap_or_else(|_| panic!("invalid qoi magic number: {name}"))
        );
        let width = u32::from_be_bytes([qoi[4], qoi[5], qoi[6], qoi[7]]);
        assert_ne!(width, 0, "width is 0");
        let height = u32::from_be_bytes([qoi[8], qoi[9], qoi[10], qoi[11]]);
        assert_ne!(height, 0, "height is 0");
        let pixels = width * height;
        assert!(
            pixels <= MAX_PIXELS,
            "image too large: {pixels}px ({width}x{height}) > {}",
            MAX_PIXELS
        );
        let channels = qoi[12] as usize;
        assert!(
            channels == 3 || channels == 4,
            "invalid channel count: {channels}"
        );
        let img_len = pixels as usize * channels;
        let mut img = vec![0; img_len];
        let colorspace = qoi[13];
        assert!(
            colorspace == SRGB || colorspace == LINEAR,
            "invalid colorspace"
        );
        let mut i = 14;
        let mut px_idx = 0;
        let mut px: [u8; 4] = [0, 0, 0, 255];
        let mut seen: [[u8; 4]; 64] = [[0, 0, 0, 0]; 64];
        while px_idx < img_len {
            let mut run = 0;
            let tag = qoi[i];
            let rgba = match tag {
                RGBA_MASK => {
                    assert!(channels == 4, "encountered RGBA chunk in RGB image");
                    let rgba = [qoi[i + 1], qoi[i + 2], qoi[i + 3], qoi[i + 4]];
                    i += 4;
                    rgba
                }
                RGB_MASK => {
                    let rgb = [qoi[i + 1], qoi[i + 2], qoi[i + 3], px[3]];
                    i += 3;
                    rgb
                }
                tag => match tag & 0xC0 {
                    INDEX_MASK => seen[tag as usize],
                    DIFF_MASK => {
                        let dr = (tag >> 4) & 0b11;
                        let dg = (tag >> 2) & 0b11;
                        let db = tag & 0b11;
                        assert!(dr <= 3, "invalid red diff: -2 <= {} <= 1", dr as i8);
                        assert!(dg <= 3, "invalid green diff: -2 <= {} <= 1", dg as i8);
                        assert!(db <= 3, "invalid blue diff: -2 <= {} <= 1", db as i8);
                        let [r, g, b, a] = px;
                        let r = r.wrapping_add(dr.wrapping_sub(2));
                        let g = g.wrapping_add(dg.wrapping_sub(2));
                        let b = b.wrapping_add(db.wrapping_sub(2));
                        [r, g, b, a]
                    }
                    LUMA_MASK => {
                        i += 1;
                        let nxt = qoi[i];
                        let dg = (tag & 0x3F).wrapping_sub(32);
                        let dr = ((nxt >> 4) & 0x0F).wrapping_sub(8);
                        let db = (nxt & 0x0F).wrapping_sub(8);
                        assert!(
                            dr as i8 >= -8 && dr as i8 <= 7,
                            "invalid red luma diff: -8 <= {} <= 7",
                            dr as i8
                        );
                        assert!(
                            dg as i8 >= -32 && dg as i8 <= 31,
                            "invalid green luma diff: -32 <= {} <= 31",
                            dg as i8
                        );
                        assert!(
                            db as i8 >= -8 && db as i8 <= 7,
                            "invalid blue luma diff: -8 <= {} <= 7",
                            db as i8
                        );
                        let [r, g, b, a] = px;
                        let r = r.wrapping_add(dr).wrapping_add(dg);
                        let g = g.wrapping_add(dg);
                        let b = b.wrapping_add(db).wrapping_add(dg);
                        [r, g, b, a]
                    }
                    RUN_MASK => {
                        let run_len = tag & 0x3F;
                        assert!(run_len < 62, "illegal run-length: {} > 61", run_len);
                        run = run_len;
                        px
                    }
                    _ => panic!(),
                },
            };
            for _ in 0..=run {
                img[px_idx..][..channels].copy_from_slice(&rgba[..channels]);
                px_idx += channels;
            }
            i += 1;
            seen[Self::hash(rgba) as usize] = rgba;
            px = rgba;
        }
        assert_eq!(
            &qoi[i..i + 8],
            &[0, 0, 0, 0, 0, 0, 0, 1],
            "image does not end with padding"
        );
        ImageData {
            img,
            width,
            height,
            channels: channels as u8,
        }
    }

    pub fn save(name: &str, img: &[u8], width: u32, height: u32, channels: u8) -> usize {
        crate::scope_time!("QOI save");
        let pixels = width * height;
        assert!(
            pixels <= MAX_PIXELS,
            "image too large: {pixels}px ({width}x{height}) > {}",
            MAX_PIXELS
        );
        let channels = channels as usize;
        assert_eq!(
            img.len() as u32,
            pixels * channels as u32,
            "img size({}) doesn't match {width}x{height}x{channels}",
            img.len()
        );
        let mut qoi = Vec::with_capacity(pixels as usize * channels);
        qoi.extend_from_slice(b"qoif");
        qoi.extend_from_slice(&width.to_be_bytes());
        qoi.extend_from_slice(&height.to_be_bytes());
        qoi.push(channels as u8);
        let colorspace = SRGB;
        qoi.push(colorspace);
        let mut i = 0;
        let mut prev = [0, 0, 0, 255];
        let mut seen: [[u8; 4]; 64] = [[0, 0, 0, 0]; 64];
        while i < img.len() {
            let mut rgba = [
                img[i],
                img[i + 1],
                img[i + 2],
                if channels == 4 { img[i + 3] } else { 255 },
            ];
            if rgba == prev {
                let mut run = 0;
                while rgba == prev && run < 62 {
                    run += 1;
                    i += channels;
                    if i >= img.len() {
                        break;
                    }
                    prev = rgba;
                    rgba = [
                        img[i],
                        img[i + 1],
                        img[i + 2],
                        if channels == 4 { img[i + 3] } else { 255 },
                    ];
                }
                run -= 1;
                assert!(run < 62, "invalid run length: {run} > 61");
                qoi.push(RUN_MASK | run);
            } else {
                let hash = Self::hash(rgba);
                if seen[hash as usize] == rgba {
                    qoi.push(INDEX_MASK | hash);
                } else {
                    seen[hash as usize] = rgba;
                    if rgba[3] == prev[3] {
                        let dr = rgba[0].wrapping_sub(prev[0]);
                        let dg = rgba[1].wrapping_sub(prev[1]);
                        let db = rgba[2].wrapping_sub(prev[2]);
                        let dru = dr.wrapping_add(2);
                        let dgu = dg.wrapping_add(2);
                        let dbu = db.wrapping_add(2);
                        if dru <= 3 && dgu <= 3 && dbu <= 3 {
                            qoi.push(DIFF_MASK | (dru << 4) | (dgu << 2) | dbu);
                        } else {
                            let dgu = dg.wrapping_add(32);
                            let dru = dr.wrapping_sub(dg).wrapping_add(8);
                            let dbu = db.wrapping_sub(dg).wrapping_add(8);
                            if dgu <= 63 && dru <= 15 && dbu <= 15 {
                                qoi.push(LUMA_MASK | dgu);
                                qoi.push((dru << 4) | dbu);
                            } else {
                                qoi.push(RGB_MASK);
                                qoi.push(rgba[0]);
                                qoi.push(rgba[1]);
                                qoi.push(rgba[2]);
                            }
                        }
                    } else {
                        assert!(channels == 4, "RGBA chunk encountered in RGB image");
                        qoi.push(RGBA_MASK);
                        qoi.push(rgba[0]);
                        qoi.push(rgba[1]);
                        qoi.push(rgba[2]);
                        qoi.push(rgba[3]);
                    }
                }
                i += channels;
                prev = rgba;
            }
        }
        qoi.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
        let qoi_len = qoi.len();
        assert!(qoi_len >= MIN_QOI_LEN, "qoi too small");
        let img_path = format!("{RES_PATH}/images/{name}.qoi");
        std::fs::write(&img_path, &qoi)
            .unwrap_or_else(|e| panic!("failed to save qoi image({}): {e}", img_path));
        qoi_len
    }

    #[inline(always)]
    fn hash(rgba: [u8; 4]) -> u8 {
        (rgba[0].wrapping_mul(3))
            .wrapping_add(rgba[1].wrapping_mul(5))
            .wrapping_add(rgba[2].wrapping_mul(7))
            .wrapping_add(rgba[3].wrapping_mul(11))
            % 64
    }

    pub fn flip_vert(img_data: &mut ImageData) -> &mut ImageData {
        let height = img_data.height as usize;
        let row_size = img_data.width as usize * img_data.channels as usize;
        for i in 0..height / 2 {
            let top_row_start = i * row_size;
            let bottom_row_start = (height - 1 - i) * row_size;
            unsafe {
                std::ptr::swap_nonoverlapping(
                    img_data.img.as_mut_ptr().add(top_row_start),
                    img_data.img.as_mut_ptr().add(bottom_row_start),
                    row_size,
                )
            };
        }
        img_data
    }

    pub fn make4(rgb: &mut [u8]) -> Vec<u8> {
        assert_eq!(rgb.len() % 3, 0, "Non-RGB image can't be made to RGBA");
        let mut rgba = vec![0u8; rgb.len() / 3 * 4];
        let rgb_chunks = unsafe { rgb.as_chunks_unchecked::<3>() };
        let rgba_chunks = unsafe { rgba.as_chunks_unchecked_mut::<4>() };
        for (rgb, rgba) in rgb_chunks.iter().zip(rgba_chunks.iter_mut()) {
            rgba[0] = rgb[0];
            rgba[1] = rgb[1];
            rgba[2] = rgb[2];
            rgba[3] = 255;
        }
        rgba
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_img_eq(a: &ImageData, b: &ImageData) {
        assert_eq!(a.img.len(), b.img.len(), "img size not equal");
        if a.img != b.img {
            println!("left img (wrong):");
            crate::print_img(&a.img, a.width, a.height, a.channels);
            println!("right img (truth):");
            crate::print_img(&b.img, b.width, b.height, a.channels);
            panic!();
        }
    }

    fn save_load(img: &ImageData) {
        Qoi::save("temp", &img.img, img.width, img.height, img.channels);
        let limg = Qoi::load("temp");
        assert_eq!(img.width, limg.width);
        assert_eq!(img.height, limg.height);
        assert_eq!(img.channels, limg.channels);
        assert_img_eq(&limg, &img);
    }

    #[test]
    fn save_load_test() {
        *crate::INIT_PATHS;
        let img = vec![
            155, 000, 000, /**/ 155, 000, 000, // run len + rgb
            000, 200, 000, /**/ 250, 168, 250, // luma
            000, 000, 155, /**/ 001, 000, 153, // diff
            000, 200, 000, /**/ 250, 168, 250, // index
        ];
        save_load(&ImageData::new(img, 2, 4, 3));

        save_load(&ImageData::new(b"AAAAAA".repeat(64), 2 * 64, 1, 3)); // big run len
        save_load(&ImageData::new(b"AAAEEE".repeat(64), 2 * 64, 1, 3)); // big luma
        save_load(&ImageData::new(b"AAABBB".repeat(64), 2 * 64, 1, 3)); // big diff
        save_load(&ImageData::new(b"QQQAAA".repeat(64), 2 * 64, 1, 3)); // big index
        save_load(&ImageData::new(
            [b"AAAAAA".repeat(64), b"EEE".repeat(63)].concat(),
            2 * 64 + 63,
            1,
            3,
        )); // big double run len

        // visualize correctly loaded/saved (hopefuly) QOI image
        let img = Qoi::load("../../../res/images/spark");
        Qoi::save("temp", &img.img, img.width, img.height, img.channels);
        let limg = Qoi::load("temp");
        assert_img_eq(&img, &limg);

        let img = Qoi::load("../../../res/images/cursor");
        Qoi::save("temp", &img.img, img.width, img.height, img.channels);
        let limg = Qoi::load("temp");
        assert_img_eq(&img, &limg);

        let img = Qoi::load("../../../res/images/spiral");
        Qoi::save("temp", &img.img, img.width, img.height, img.channels);
        let limg = Qoi::load("temp");
        assert_img_eq(&img, &limg);
    }
}
