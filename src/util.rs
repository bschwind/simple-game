use std::collections::VecDeque;

struct RollingAverage {
    items: VecDeque<f64>,
}

impl RollingAverage {
    fn new(size: usize) -> Self {
        Self { items: VecDeque::with_capacity(size) }
    }

    fn insert(&mut self, item: f64) {
        if self.items.len() >= self.items.capacity() {
            self.items.pop_front();
        }

        self.items.push_back(item);
    }

    fn avg(&self) -> f64 {
        self.items.iter().sum::<f64>() / self.items.len() as f64
    }
}

pub struct FPSCounter {
    last_call: std::time::Instant,
    rolling_avg: RollingAverage,
}

impl Default for FPSCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl FPSCounter {
    pub fn new() -> Self {
        Self { last_call: std::time::Instant::now(), rolling_avg: RollingAverage::new(30) }
    }

    pub fn tick(&mut self) {
        let elapsed = self.last_call.elapsed();
        let fps: f64 = 1_000_000.0 / elapsed.as_micros() as f64;
        self.rolling_avg.insert(fps);
        self.last_call = std::time::Instant::now();
    }

    pub fn fps(&self) -> usize {
        self.rolling_avg.avg() as usize
    }
}
