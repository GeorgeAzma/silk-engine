use crate::util::{
    image_loader::{ImageData, ImageFormat},
    reader::Reader,
    writer::Writer,
};

pub struct Bmp;

#[repr(C, packed)]
struct Head {
    magic: u16,
    file_size: u32,
    reserved: u32,
    off: u32,
    size: u32,
    width: u32,
    height: u32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    compressed_img_size: u32,
    x_px_per_m: u32,
    y_px_per_m: u32,
    colors_used: u32,
    important_colors: u32,
}

const BMP_HEAD_LEN: usize = size_of::<Head>();

impl ImageFormat for Bmp {
    fn load(name: &str) -> ImageData {
        let data = std::fs::read(format!("res/images/{name}.bmp"))
            .unwrap_or_else(|_| panic!("bmp image not found: {name}"));
        let mut reader = Reader::new(&data);
        let magic = reader.read16().to_le_bytes();
        assert_eq!(magic, *b"BM", "invalid magic number for BMP");
        let file_size = reader.read32();
        assert!(
            file_size as usize > BMP_HEAD_LEN,
            "file is too small to be valid BMP"
        );
        reader.skip(4); // reserved
        let off = reader.read32();
        assert!(
            off >= BMP_HEAD_LEN as u32,
            "expected bmp pixel data start offset to be >= 54"
        );

        let size = reader.read32();
        assert_eq!(size, 40, "expected info header size to be 40");
        let width = reader.read32();
        let height = reader.read32();
        reader.skip(2); // planes
        // 1 monochrome, 4 pallete, 8 pallete, 16 RGB, 24 RGB
        let bit_count = reader.read16();
        // 0 = None, 1 = Run len encoding 8, 2 = run len encoding 4
        let compression = reader.read32();
        assert_eq!(compression, 0, "BMP compression not supported");
        let _compressed_img_size = reader.read32();
        reader.skip(8); // x/y pixels per meter
        let colors_used = reader.read32();
        reader.skip(4); // important colors
        if bit_count <= 8 {
            let _color_table = reader.read(4 * colors_used as usize);
        }
        reader.goto(off as usize);
        let row_size = ((width as usize * bit_count as usize + 31) / 32) * 4;
        let channels = match bit_count {
            1 | 4 | 8 => 1,
            16 | 24 => 3,
            32 => 4,
            _ => panic!("invalid bit count: {bit_count}, expected 1, 4, 8, 16, 24, 32"),
        };
        let mut img = Vec::with_capacity(width as usize * height as usize * channels as usize);
        for _ in 0..height {
            let row = reader.read(row_size);
            match bit_count {
                8 => {
                    img.extend_from_slice(&row[..width as usize]);
                }
                24 => {
                    for chunk in row.chunks_exact(3) {
                        img.extend([chunk[2], chunk[1], chunk[0]]); // BMP stores as BGR
                    }
                }
                32 => {
                    for chunk in row.chunks_exact(4) {
                        img.extend([chunk[2], chunk[1], chunk[0], chunk[3]]);
                    }
                }
                _ => unimplemented!(),
            }
        }

        ImageData::new(img, width, height, channels)
    }

    fn save(name: &str, img: &[u8], width: u32, height: u32, channels: u8) {
        assert!(!img.is_empty(), "img was empty");
        assert_ne!(width, 0, "width was 0");
        assert_ne!(height, 0, "height was 0");
        let colors_used = if channels == 1 { 256u32 } else { 0u32 };
        let off = BMP_HEAD_LEN as u32 + colors_used * 4;
        let bit_count = match channels {
            1 => 8,
            3 => 24,
            4 => 32,
            _ => panic!("BMP does not support channel count of {channels}"),
        };
        let row_size = (width * bit_count as u32 + 31) / 32 * 4;
        let padded_img_len = row_size * height;
        let file_size = (off + padded_img_len) as usize;
        let mut writer = Writer::new(file_size);
        writer.write(b"BM"); // magic
        writer.write32(file_size as u32);
        writer.skip(4); // reserved
        writer.write32(off); // off
        writer.write32(40); // info header size
        writer.write32(width);
        writer.write32(height);
        writer.write16(1); // planes
        writer.write16(bit_count);
        writer.write32(0); // compression
        writer.write32(padded_img_len); // compressed image size
        writer.write32(0); // x pixels per meter
        writer.write32(0); // y pixels per meter
        writer.write32(colors_used);
        writer.write32(0); // important colors
        assert_eq!(writer.idx(), BMP_HEAD_LEN, "bmp header size is incorrect");
        if bit_count <= 8 {
            for i in 0..colors_used {
                writer.write8(i as u8); // B
                writer.write8(i as u8); // G
                writer.write8(i as u8); // R
                writer.skip(1);
            }
        }
        assert_eq!(writer.idx(), off as usize, "bmp off is incorrect");
        let pad = (row_size - width * channels as u32) as usize;
        for row in img.chunks_exact(width as usize * channels as usize) {
            match bit_count {
                8 => writer.write(row),
                24 => {
                    for chunk in row.chunks_exact(3) {
                        writer.write8(chunk[2]); // B
                        writer.write8(chunk[1]); // G
                        writer.write8(chunk[0]); // R
                    }
                }
                32 => {
                    for chunk in row.chunks_exact(4) {
                        writer.write8(chunk[2]); // B
                        writer.write8(chunk[1]); // G
                        writer.write8(chunk[0]); // R
                        writer.write8(chunk[3]); // A
                    }
                }
                _ => unreachable!(),
            }
            writer.skip(pad);
        }
        assert_eq!(writer.idx(), file_size, "BMP file size is incorrect");
        let path = format!("res/images/{name}.bmp");
        let bytes = writer.finish();
        std::fs::write(path, bytes).unwrap();
    }
}
