#version 430

const vec2 verts[4] = vec2[4](
    vec2(-1.0, -1.0),
    vec2(-1.0, 1.0),
    vec2(1.0, -1.0),
    vec2(1.0, 1.0)
);

uniform float u_angle;
out vec2 v_pos;
void main() {
    gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
    v_pos = verts[gl_VertexID];
}