import colorsys

COLOR_MAP = {
    i: tuple(
        int(c * 255) for c in colorsys.hsv_to_rgb((i * 137.5) % 360 / 360.0, 1.0, 1.0)
    )
    for i in range(100)
}

COLOR_MAP_RGB_STRING = {
    i: f"rgb({color[0]},{color[1]},{color[2]})" for i, color in COLOR_MAP.items()
}
