#version 430

layout(std430, binding = 1) buffer DataOut {
    vec2 bounds[];
};

uniform uint u_num_pixels;

out vec4 out_color;
in vec2 v_pos;
void main() {
    uint cur_pixel = uint((v_pos.x + 1.0) / 2.0 * float(u_num_pixels));
    if (v_pos.y < bounds[cur_pixel].x || v_pos.y > bounds[cur_pixel].y) {
        out_color = vec4(0.0, 0.0, 0.0, 1.0);
    } else {
        out_color = vec4(1.0, 1.0, 1.0, 1.0);
    }
}