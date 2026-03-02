// Test shader: magenta overlay. Replaces color with solid magenta, preserving alpha.
vec4 custom_postprocess() {
    float a = texture2D(tex, v_coords).a;
    return vec4(1.0, 0.0, 1.0, a);
}
