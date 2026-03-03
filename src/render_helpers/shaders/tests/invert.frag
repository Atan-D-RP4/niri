// Test shader: color inversion. Inverts RGB channels, preserves alpha.
vec4 custom_postprocess(vec4 input_color) {
    return vec4(1.0 - input_color.rgb, input_color.a);
}
