// Test shader: color inversion. Inverts RGB channels, preserves alpha.
vec4 custom_postprocess() {
    vec4 color = texture2D(tex, v_coords);
    return vec4(1.0 - color.rgb, color.a);
}
