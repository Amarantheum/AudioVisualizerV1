#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pos: [f32; 2],
}

impl Vertex {
    pub fn new(pos: [f32; 2]) -> Self {
        Self {
            pos,
        }
    }
    pub fn vertices_from_array(arr: &[[f32; 2]]) -> Vec<Vertex> {
        let mut out = Vec::with_capacity(arr.len());
        for v in arr {
            out.push(Vertex::new(*v));
        }
        out
    }
}

implement_vertex!(Vertex, pos);