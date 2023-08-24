#version 430

layout(std430, binding = 1) buffer DataOut {
    vec2 bounds[];
};

out vec4 out_color;
in vec2 v_pos;
void main() {
    float x = gl_FragCoord.x / 300.0 * 10.0;
    uint index = uint(floor(x));
    float y = gl_FragCoord.y / 300.0;
    if (y > bounds[index].x) {
        out_color = vec4(1.0, 0.0, 0.0, 1.0);
    } else {
        out_color = vec4(0.0, 1.0, 0.0, 1.0);
    }
}