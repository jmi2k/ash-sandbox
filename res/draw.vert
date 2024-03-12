#version 450

layout(location = 0) out vec4 v_Color;

void main() {
    gl_Position = vec4(
        float(gl_VertexIndex == 1),
        float(gl_VertexIndex == 2),
        0,
        1
    );

    v_Color = vec4(
        float(gl_VertexIndex == 0),
        float(gl_VertexIndex == 1),
        float(gl_VertexIndex == 2),
        1
    );
}
