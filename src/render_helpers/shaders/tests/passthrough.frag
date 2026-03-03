// Test shader: passthrough. Returns input unchanged, identical to default no-op.
vec4 custom_postprocess(vec4 input_color) {
    return input_color;
}
