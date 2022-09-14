use glium::{VertexBuffer, IndexBuffer, Program};

use crate::graphics::vertex::Vertex;

pub mod spectrum;

pub trait AudioGraphic {
    fn get_data_buffers(&self) -> (VertexBuffer<Vertex>, IndexBuffer<u32>);
    fn get_program(&self) -> Program;
}