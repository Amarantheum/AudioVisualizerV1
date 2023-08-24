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

    #[allow(unused)]
    pub fn get_raw(&self) -> &[f32] {
        &self.buffer
    }
}

impl Index<usize> for RingBuffer {
    type Output = f32;

    fn index(&self, value: usize) -> &Self::Output {
        &self.buffer[self.back + value]
    }
}