use crate::BUFFER_SIZE;
use std::ops::Index;

pub struct RingBuffer {
    back: usize,
    buffer: [f32; BUFFER_SIZE],
}

impl RingBuffer {
    pub fn new() -> Self {
        Self {
            back: 0,
            buffer: [0.0; BUFFER_SIZE],
        }
    }

    pub fn push_back(&mut self, value: f32) {
        self.buffer[self.back] = value;
        self.back += 1;
        self.back &= BUFFER_SIZE - 1;
    }

    #[allow(unused)]
    pub fn get_vec(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(BUFFER_SIZE);
        for i in 0..BUFFER_SIZE {
            v.push(self.buffer[(self.back + i) & (BUFFER_SIZE - 1)])
        }
        v
    }

    /// Get the most recent `count` samples (newest first in time order)
    pub fn get_recent(&self, count: usize) -> Vec<f32> {
        let count = count.min(BUFFER_SIZE);
        let mut v = Vec::with_capacity(count);
        // back points to where next sample will be written, so back-1 is most recent
        for i in 0..count {
            let idx = (self.back + BUFFER_SIZE - count + i) & (BUFFER_SIZE - 1);
            v.push(self.buffer[idx]);
        }
        v
    }

    #[allow(unused)]
    pub fn get_raw(&self) -> &[f32] {
        &self.buffer
    }

    /// Get buffer in raw storage order (for sweep mode display)
    /// Returns (data, write_position) where write_position is where new data is being written
    pub fn get_raw_with_position(&self) -> (Vec<f32>, usize) {
        (self.buffer.to_vec(), self.back)
    }
}

impl Index<usize> for RingBuffer {
    type Output = f32;

    fn index(&self, value: usize) -> &Self::Output {
        &self.buffer[self.back + value]
    }
}