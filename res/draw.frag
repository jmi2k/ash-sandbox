#version 450

layout(push_constant) uniform Pushs {
    layout(offset = 0) bool is_srgb;
} p;

layout(location = 0) in vec4 v_Color;
layout(location = 0) out vec4 f_Color;

vec3 linear2srgb(vec3 linear) {
    // IEC 61966-2-1:1999/AMD1:2003
    bvec3 cutoff = lessThanEqual(linear, vec3(.0031308));
    vec3 higher = vec3(1.055) * pow(linear, vec3(1. / 2.4)) - vec3(.055);
    vec3 lower = linear * vec3(12.92);

    return mix(higher, lower, cutoff);
}

void main() {
    f_Color = mix(vec4(linear2srgb(v_Color.rgb), v_Color.a), v_Color, int(p.is_srgb));
}
