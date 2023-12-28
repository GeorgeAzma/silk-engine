pub struct Cooldown {
    timer: std::time::Instant,
    delay: std::time::Duration,
}

impl Cooldown {
    pub fn new(delay: std::time::Duration) -> Self {
        Self {
            timer: std::time::Instant::now(),
            delay,
        }
    }

    pub fn reset(&mut self) {
        self.timer = std::time::Instant::now();
    }

    pub fn ready(&self) -> bool {
        self.timer.elapsed() >= self.delay
    }
}
