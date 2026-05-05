// Build a 32x32 RGBA buffer for the tray icon: horizontal battery shape with colored fill.

const W: u32 = 32;
const H: u32 = 32;

#[derive(Clone, Copy)]
struct Rgba(u8, u8, u8, u8);

const TRANSPARENT: Rgba = Rgba(0, 0, 0, 0);
const WHITE: Rgba = Rgba(255, 255, 255, 255);

fn fill_color(pct: Option<u8>) -> Rgba {
    match pct {
        None => Rgba(150, 150, 150, 255),
        Some(p) if p < 20 => Rgba(255, 70, 70, 255),
        Some(p) if p < 60 => Rgba(255, 165, 0, 255),
        Some(_) => Rgba(80, 220, 100, 255),
    }
}

pub fn build_icon_rgba(pct: Option<u8>) -> Vec<u8> {
    let mut buf = vec![0u8; (W * H * 4) as usize];

    // Battery body outline: rectangle from (2,8) to (25,23), 2px stroke
    // Cap: filled rectangle from (26,12) to (28,19)
    // Inner area: (4,10) to (23,21)

    let put = |buf: &mut Vec<u8>, x: u32, y: u32, c: Rgba| {
        if x < W && y < H {
            let i = ((y * W + x) * 4) as usize;
            buf[i] = c.0;
            buf[i + 1] = c.1;
            buf[i + 2] = c.2;
            buf[i + 3] = c.3;
        }
    };
    let fill_rect =
        |buf: &mut Vec<u8>, x0: u32, y0: u32, x1: u32, y1: u32, c: Rgba| {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    put(buf, x, y, c);
                }
            }
        };
    let stroke_rect =
        |buf: &mut Vec<u8>, x0: u32, y0: u32, x1: u32, y1: u32, c: Rgba, w: u32| {
            // Top and bottom bars
            for x in x0..=x1 {
                for d in 0..w {
                    put(buf, x, y0 + d, c);
                    put(buf, x, y1 - d, c);
                }
            }
            // Left and right bars
            for y in y0..=y1 {
                for d in 0..w {
                    put(buf, x0 + d, y, c);
                    put(buf, x1 - d, y, c);
                }
            }
        };

    // Clear
    for px in buf.chunks_exact_mut(4) {
        px[0] = 0;
        px[1] = 0;
        px[2] = 0;
        px[3] = 0;
    }

    // Battery outline (white, 2px)
    stroke_rect(&mut buf, 2, 8, 25, 23, WHITE, 2);

    // Battery cap (white, filled)
    fill_rect(&mut buf, 26, 12, 28, 19, WHITE);

    // Fill / placeholder
    let color = fill_color(pct);
    if let Some(p) = pct {
        // Inner fill area: x 4..=23 (width 20), y 10..=21 (height 12)
        let inner_w = 20u32;
        let fill_w = ((inner_w as u32 * p as u32) / 100).max(0) as u32;
        if fill_w > 0 {
            fill_rect(&mut buf, 4, 10, 4 + fill_w - 1, 21, color);
        }
    } else {
        // Offline: draw a "?" using a few pixels (very crude)
        // Draw a small grey '?' in the inner area: top arc + dot
        let q = color;
        // top curve
        for x in 9..=14 {
            put(&mut buf, x, 11, q);
        }
        put(&mut buf, 15, 12, q);
        put(&mut buf, 15, 13, q);
        put(&mut buf, 14, 14, q);
        put(&mut buf, 13, 15, q);
        put(&mut buf, 12, 16, q);
        put(&mut buf, 12, 17, q);
        // dot
        put(&mut buf, 12, 19, q);
        put(&mut buf, 12, 20, q);
    }

    buf
}

pub fn icon_dim() -> (u32, u32) {
    (W, H)
}
