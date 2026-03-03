// Test shader: magenta overlay. Replaces color with solid magenta, preserving alpha.
vec4 custom_postprocess(vec4 input_color) {
    return vec4(1.0, 0.0, 1.0, input_color.a);
}
