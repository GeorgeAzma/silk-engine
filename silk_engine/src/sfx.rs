use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use cpal::{
    BufferSize, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::util::ExtraFns;

#[derive(Clone)]
pub struct Source {
    pub samples: Vec<f32>,
    pub pos: usize,
    pub loops: usize,
    pub volume: f32,
    pub paused: bool,
    // lr: f32, // 0 = left | 0.5 = both | 1 = right
    uid: usize,
    goal_volume: f32,
    effective_volume: f32,
}

impl Source {
    pub fn from_vec(samples: Vec<f32>) -> Self {
        Self {
            samples,
            pos: 0,
            loops: 1,
            volume: 1.0,
            paused: false,
            uid: 0,
            goal_volume: 1.0,
            effective_volume: 0.0,
        }
    }

    pub fn new(samples: &[f32]) -> Self {
        Self::from_vec(samples.to_vec())
    }

    pub fn load(_name: &str) -> Self {
        todo!("custom wav loading")
    }

    pub fn loops(&mut self, loops: usize) -> &mut Self {
        self.loops = loops;
        self
    }

    pub fn volume(&mut self, volume: f32) -> &mut Self {
        self.volume = volume;
        self
    }

    pub fn pause(&mut self) -> &mut Self {
        self.paused = !self.paused;
        self.goal_volume = if self.paused { 0.0 } else { self.volume };
        self
    }

    /// returns source's uid
    pub fn play(&mut self, sfx: &Sfx) -> usize {
        sfx.play(self)
    }

    fn update(&mut self, dt: f32) {
        self.effective_volume = self
            .effective_volume
            .lerp(self.goal_volume, (dt * 150.0).saturate());
    }
}

pub struct Sfx {
    stream: Stream,
    config: StreamConfig,
    /// sound samples for each active sound
    sources: Arc<Mutex<Vec<Source>>>,
    uid: AtomicUsize,
}

impl Sfx {
    pub(crate) fn new() -> Self {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("speaker not found");

        let mut config = device.default_output_config().unwrap().config();

        const BUF_SIZE: u32 = 1024;
        config.buffer_size = match config.buffer_size {
            BufferSize::Default => BufferSize::Fixed(BUF_SIZE),
            BufferSize::Fixed(size) => BufferSize::Fixed(BUF_SIZE.min(size)),
        };

        let sources = Arc::new(Mutex::new(Vec::<Source>::new()));
        let sources_clone = sources.clone();

        // if at end or start or before pause or after pause
        let stream = device
            .build_output_stream(
                &config,
                move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut sources = sources_clone.lock().unwrap();
                    let ch = config.channels as f32;
                    let dt = 1.0 / config.sample_rate.0 as f32 / ch;
                    let fade = 1200.0 * ch;
                    for sample in output.iter_mut() {
                        *sample = 0.0;
                        for src in sources.iter_mut() {
                            if src.loops == 0 {
                                break;
                            }
                            let pos = src.pos as f32;
                            if pos >= src.samples.len() as f32 - fade {
                                src.goal_volume = 0.0;
                            }
                            if pos < fade && !src.paused {
                                src.goal_volume = src.volume;
                            }
                            src.update(dt);

                            if src.pos < src.samples.len() {
                                *sample += src.samples[src.pos] * src.effective_volume;
                                if !(src.paused && src.effective_volume < 1e-5) {
                                    src.pos += 1;
                                }
                            } else {
                                src.pos = 0;
                                src.loops = src.loops.saturating_sub(1);
                                src.goal_volume = src.volume;
                            }
                        }
                    }
                    sources.retain(|src| src.loops > 0);
                },
                |e| crate::err!("audio error: {e}"),
                None,
            )
            .unwrap();

        stream.play().unwrap();

        Self {
            stream,
            config,
            sources,
            uid: 0.into(),
        }
    }

    /// returns source's uid
    pub fn play(&self, source: &mut Source) -> usize {
        let uid = self.uid.fetch_add(1, Ordering::SeqCst);
        let mut sources = self.sources.lock().unwrap();
        source.uid = uid;
        sources.push(source.to_owned());
        return uid;
    }

    pub fn load(&self, _name: &str) {
        todo!("custom wav loading")
    }

    pub fn gen_mono(&self, secs: f32, mut f: impl FnMut(f32) -> f32) -> Source {
        let dt = 1.0 / self.sample_rate() as f32;
        let mut t = 0.0;
        let ch = self.channels() as usize;
        let mut samples = Vec::with_capacity((secs / dt).ceil() as usize * ch);
        while t <= secs {
            for _ in 0..ch {
                samples.push(f(t));
            }
            t += dt;
        }
        Source::from_vec(samples)
    }

    pub fn gen_stereo(&self, secs: f32, mut f: impl FnMut(f32) -> (f32, f32)) -> Source {
        let dt = 1.0 / self.sample_rate() as f32;
        let mut t = 0.0;
        let ch = self.channels() as usize;
        let mut samples = Vec::with_capacity((secs / dt).ceil() as usize * ch);
        while t <= secs {
            let (l, r) = f(t);
            if self.channels() >= 2 {
                samples.push(l);
                samples.push(r);
            } else {
                samples.push((l + r) * 0.5);
            }
            t += dt;
        }
        Source::from_vec(samples)
    }

    fn source_fn(&self, uid: usize, f: impl Fn(&mut Source)) {
        if let Some(src) = self
            .sources
            .lock()
            .unwrap()
            .iter_mut()
            .find(|src| src.uid == uid)
        {
            f(src)
        }
    }

    pub fn looping(&self, uid: usize) {
        self.source_fn(uid, |src| {
            src.loops(usize::MAX);
        });
    }

    pub fn loops(&self, uid: usize, loops: usize) {
        self.source_fn(uid, |src| {
            src.loops(loops);
        });
    }

    pub fn pause(&self, uid: usize) {
        self.source_fn(uid, |src| {
            src.pause();
        });
    }

    pub fn volume(&self, uid: usize, volume: f32) {
        self.source_fn(uid, |src| src.volume = volume);
    }

    pub fn pause_stream(&self) {
        self.stream
            .pause()
            .unwrap_or_else(|e| crate::warn!("audio device does not support pausing: {e}"));
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    pub fn buffer_size(&self) -> u32 {
        match self.config.buffer_size {
            BufferSize::Default => unreachable!(),
            BufferSize::Fixed(size) => size,
        }
    }
}
