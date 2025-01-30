pub struct Cooldown {
    timer: Instant,
    pub delay: Duration,
}

#[allow(unused)]
impl Cooldown {
    pub fn new(delay: Duration) -> Self {
        Self {
            timer: Instant::now(),
            delay,
        }
    }

    pub fn sec(secs: f32) -> Self {
        Self::new(Duration::from_secs_f32(secs))
    }

    pub fn ms(ms: u32) -> Self {
        Self::new(Duration::from_millis(ms as u64))
    }

    pub fn us(us: u32) -> Self {
        Self::new(Duration::from_micros(us as u64))
    }

    pub fn ns(ns: u32) -> Self {
        Self::new(Duration::from_nanos(ns as u64))
    }

    pub fn ready(&self) -> bool {
        self.dt() >= self.delay
    }

    pub fn dt(&self) -> Duration {
        self.timer.elapsed()
    }

    pub fn reset(&mut self) {
        self.timer = Instant::now();
    }

    pub fn next(&mut self) {
        self.timer += self.delay;
    }
}
