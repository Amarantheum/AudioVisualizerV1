#version 430

layout(std140) buffer SpectrumIn {
    vec4 amps[];
};

in vec2 frag_pos;
in vec2 resolution;
out vec4 color;

int get_index(int frag_index) {
    return frag_index / 4;
}

int get_offset(int frag_index) {
    return frag_index % 4;
}

void main() {
    int frag_index = int(gl_FragCoord.x);
    if (frag_pos.y < amps[get_index(frag_index)][get_offset(frag_index)]) {
        color = vec4(1.0, 0.0, 1.0, 1.0);
    } else {
        color = vec4(0.0);
    }
    
}