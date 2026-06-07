#!/usr/bin/env python3
"""Generate Vosi logo PNGs: transparent outside, solid white capsule inside."""
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "assets" / "vosi-logo-1024-transparent.png"
TRAY_OUT = ROOT / "assets" / "vosi-tray-1024-transparent.png"
ICONS = ROOT / "src-tauri" / "icons"


def draw_logo(size: int, cap_w_ratio: float, cap_h_ratio: float) -> Image.Image:
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    cx, cy = size // 2, size // 2

    cap_w = int(size * cap_w_ratio)
    cap_h = int(size * cap_h_ratio)
    x0, y0 = cx - cap_w // 2, cy - cap_h // 2
    x1, y1 = cx + cap_w // 2, cy + cap_h // 2
    radius = cap_h // 2

    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        (x0 + 3, y0 + 5, x1 + 3, y1 + 5), radius=radius, fill=(0, 0, 0, 70)
    )
    im = Image.alpha_composite(im, shadow)
    draw = ImageDraw.Draw(im)

    draw.rounded_rectangle((x0, y0, x1, y1), radius=radius, fill=(255, 255, 255, 255))
    border = max(3, size // 128)
    draw.rounded_rectangle(
        (x0, y0, x1, y1), radius=radius, outline=(120, 120, 128, 220), width=border
    )

    inner_y = cy
    dot_r = max(6, int(cap_h * 0.13))
    bar_w = max(8, int(cap_h * 0.11))
    bar_gap = max(6, int(cap_h * 0.07))
    dot_bar_gap = max(8, int(cap_h * 0.09))
    bar_heights = [int(cap_h * 0.36), int(cap_h * 0.24), int(cap_h * 0.44)]

    group_w = dot_r * 2 + dot_bar_gap + 3 * bar_w + 2 * bar_gap
    cursor_x = cx - group_w // 2

    dot_cx = cursor_x + dot_r
    draw.ellipse(
        (dot_cx - dot_r, inner_y - dot_r, dot_cx + dot_r, inner_y + dot_r),
        fill=(255, 59, 48, 255),
    )

    cursor_x += dot_r * 2 + dot_bar_gap
    for h in bar_heights:
        draw.rounded_rectangle(
            (cursor_x, inner_y - h // 2, cursor_x + bar_w, inner_y + h // 2),
            radius=bar_w // 2,
            fill=(10, 132, 255, 255),
        )
        cursor_x += bar_w + bar_gap

    return im


def tint(img: Image.Image, color: tuple[int, int, int], strength: float) -> Image.Image:
    out = img.copy()
    px = out.load()
    for y in range(out.height):
        for x in range(out.width):
            r, g, b, a = px[x, y]
            if a == 0:
                continue
            px[x, y] = (
                min(255, int(r * (1 - strength) + color[0] * strength)),
                min(255, int(g * (1 - strength) + color[1] * strength)),
                min(255, int(b * (1 - strength) + color[2] * strength)),
                a,
            )
    return out


def main() -> None:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    ICONS.mkdir(parents=True, exist_ok=True)

    # App / bundle icon — current approved proportions.
    app_logo = draw_logo(1024, cap_w_ratio=0.82, cap_h_ratio=0.58)
    app_logo.save(OUT)
    app_logo.resize((512, 512), Image.Resampling.LANCZOS).save(ICONS / "icon.png")

    # Menu bar tray — larger fill so it matches neighbor icon height.
    tray_logo = draw_logo(1024, cap_w_ratio=0.94, cap_h_ratio=0.78)
    tray_logo.save(TRAY_OUT)
    tray = tray_logo.resize((512, 512), Image.Resampling.LANCZOS)
    tray.save(ICONS / "icon-idle.png")
    tint(tray, (255, 59, 48), 0.42).save(ICONS / "icon-recording.png")
    tint(tray, (255, 159, 10), 0.38).save(ICONS / "icon-warning.png")

    print(f"app icon -> {OUT}, {ICONS / 'icon.png'}")
    print(f"tray icons -> {TRAY_OUT}, icon-idle/recording/warning.png")


if __name__ == "__main__":
    main()
