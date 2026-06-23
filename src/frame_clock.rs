pub struct FrameClock {
    last_frame: instant::Instant,
    pub dt: instant::Duration,
    pub elapsed: instant::Duration,
    pub frame_index: u64,
}

impl FrameClock {
    pub fn new() -> Self {
        Self {
            last_frame: instant::Instant::now(),
            dt: instant::Duration::ZERO,
            elapsed: instant::Duration::ZERO,
            frame_index: 0,
        }
    }

    pub fn tick(&mut self) {
        let now = instant::Instant::now();
        self.dt = now - self.last_frame;
        self.last_frame = now;
        self.elapsed += self.dt;
        self.frame_index += 1;
    }
}
