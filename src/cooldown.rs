pub struct Cooldown {
    timer: std::time::Instant,
    pub delay: std::time::Duration,
}

impl Cooldown {
    pub fn new(delay: std::time::Duration) -> Self {
        Self {
            timer: std::time::Instant::now(),
            delay,
        }
    }

    pub fn ready(&self) -> bool {
        self.dt() >= self.delay
    }

    pub fn dt(&self) -> std::time::Duration {
        self.timer.elapsed()
    }

    pub fn reset(&mut self) {
        self.timer = std::time::Instant::now();
    }

    pub fn next(&mut self) {
        self.timer += self.delay;
    }
}
