use crate::{bmp::Bmp, qoi::Qoi};

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

pub trait ImageFormat {
    fn load(name: &str) -> ImageData;
    fn save(name: &str, img: &[u8], width: u32, height: u32, channels: u8);
}

pub struct ImageLoader;

impl ImageLoader {
    pub fn flip_vert(img_data: &mut ImageData) {
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

    pub fn load(file_name: &str) -> ImageData {
        let dot_pos = file_name.rfind('.').unwrap();
        let file_ext = &file_name[dot_pos..];
        let name = &file_name[..dot_pos];
        match file_ext {
            "qoi" => {
                let mut img_data = Qoi::load(name);
                ImageLoader::flip_vert(&mut img_data);
                img_data
            }
            "bmp" => Bmp::load(name),
            _ => panic!("unsupported image file extension: {file_ext}"),
        }
    }
}
