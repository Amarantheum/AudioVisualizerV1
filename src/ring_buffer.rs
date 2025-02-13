use crate::BUFFER_SIZE;
use std::ops::Index;

pub struct RingBuffer {
    back: usize,
    buffer: [f32; BUFFER_SIZE / 2],
}

#[allow(unused)]
impl RingBuffer {
    pub fn new() -> Self {
        Self {
            back: 0,
            buffer: [0.0; BUFFER_SIZE / 2],
        }
    }

    pub fn push_back(&mut self, value: f32) {
        self.buffer[self.back] = value;
        self.back += 1;
        // WARN: coupled with buffer size
        self.back &= BUFFER_SIZE / 2 - 1;
    }

    pub fn get_vec(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(BUFFER_SIZE >> 1);
        for i in 0..BUFFER_SIZE / 2 {
            // WARN: coupled with buffer size
            v.push(self.buffer[(self.back + i) & (BUFFER_SIZE / 2 - 1)])
        }
        v
    }
}

impl Index<usize> for RingBuffer {
    type Output = f32;

    fn index(&self, value: usize) -> &Self::Output {
        &self.buffer[self.back + value]
    }
}