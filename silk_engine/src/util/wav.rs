use crate::{
    RES_PATH,
    sfx::AudioData,
    util::{Reader, Writer},
};

#[derive(Default)]
struct Fmt {
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
}

pub struct Wav;

impl Wav {
    pub fn load(name: &str) -> AudioData {
        let data = std::fs::read(format!("{RES_PATH}/audio/{name}.wav"))
            .unwrap_or_else(|_| panic!("wav file not found: {name}"));
        let mut reader = Reader::new(&data);
        // Read WAVE header
        let magic = reader.read32().to_le_bytes();
        assert_eq!(magic, *b"RIFF", "invalid magic number for WAV: {name}");
        let _size = reader.read32();
        let format = reader.read32().to_le_bytes();
        assert_eq!(format, *b"WAVE", "unexpected WAV format: {name}");

        // Read Subchunks
        let mut fmt = Fmt::default();

        while reader.remaining() > 8 {
            let subchunk_id = reader.read32().to_le_bytes();
            let subchunk_size = reader.read32();
            match &subchunk_id {
                b"fmt " => {
                    assert_eq!(subchunk_size, 16);
                    let audio_fmt = reader.read16(); // 1 = PCM
                    assert_eq!(audio_fmt, 1, "compressed WAV formats are unsupported");
                    let channels = reader.read16();
                    let sample_rate = reader.read32();
                    let byte_rate = reader.read32();
                    let block_align = reader.read16();
                    let bits_per_sample = reader.read16();
                    assert_eq!(
                        byte_rate,
                        sample_rate * channels as u32 * bits_per_sample as u32 / 8
                    );
                    assert_eq!(block_align, channels * bits_per_sample / 8);
                    if audio_fmt != 1 {
                        let extra_param_size = reader.read16();
                        reader.skip(extra_param_size as usize);
                    }
                    fmt = Fmt {
                        channels,
                        sample_rate,
                        bits_per_sample,
                    };
                }
                b"data" => {
                    let data = reader.read(subchunk_size as usize);
                    let samples: Vec<f32> = match fmt.bits_per_sample {
                        8 => data.iter().map(|&s| s as f32 / i8::MAX as f32).collect(),
                        16 => data
                            .chunks_exact(2)
                            .map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / i16::MAX as f32)
                            .collect(),
                        32 => data
                            .chunks_exact(4)
                            .map(|s| {
                                i32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f32
                                    / i32::MAX as f32
                            })
                            .collect(),
                        _ => panic!("invalid WAV bit-depth"),
                    };

                    return AudioData {
                        samples,
                        sample_rate: fmt.sample_rate,
                        channels: fmt.channels,
                    };
                }
                _ => {
                    reader.skip(subchunk_size as usize);
                }
            }
        }
        panic!("invalid WAV file: {name}")
    }

    pub fn save(name: &str, samples: &[f32], sample_rate: u32, channels: u16) {
        let size = 12 + 24 + (8 + samples.len() * 2);
        let mut writer = Writer::new(size);
        writer.write32(u32::from_le_bytes(*b"RIFF"));
        writer.write32(size as u32 - 8); // placeholder for file size
        writer.write32(u32::from_le_bytes(*b"WAVE"));

        // Write fmt subchunk
        writer.write32(u32::from_le_bytes(*b"fmt "));
        writer.write32(16); // subchunk size for PCM
        writer.write16(1); // audio format (1 = PCM)
        writer.write16(channels);
        writer.write32(sample_rate);
        let byte_rate = sample_rate * channels as u32 * 16 / 8;
        writer.write32(byte_rate);
        let block_align = channels * 16 / 8;
        writer.write16(block_align);
        writer.write16(16); // bits per sample

        // Write data subchunk
        writer.write32(u32::from_le_bytes(*b"data"));
        writer.write32((samples.len() * 2) as u32); // Subchunk size
        for &sample in samples {
            writer.write16(
                (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16 as u16,
            );
        }
        assert_eq!(writer.idx(), size, "invalid WAV file size");
        std::fs::write(format!("{RES_PATH}/audio/{name}.wav"), writer.finish())
            .unwrap_or_else(|_| panic!("failed to write wav file: {name}"));
    }
}
