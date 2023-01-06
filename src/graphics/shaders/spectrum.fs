#version 430

in vec2 frag_pos;

out vec4 color;

void main() {
    if (frag_pos.y < 0) {
        color = vec4(1.0, 0.0, 1.0, 1.0);
    } else {
        color = vec4(0.0);
    }
    
}