// Test shader: passthrough. Returns input unchanged, identical to default no-op.
vec4 custom_postprocess() {
    return texture2D(tex, v_coords);
}
