// 32x32 RGBA tray icon: a green "V" outline with battery-level fill inside.
// The V outline stays green (Viper brand). The fill drops with battery level
// and shifts colour as it gets low.

const W: u32 = 32;
const H: u32 = 32;

#[derive(Clone, Copy)]
struct Rgba(u8, u8, u8, u8);

const OUTLINE: Rgba = Rgba( 70, 220,  90, 255); // brand green, always
const OFFLINE: Rgba = Rgba(120, 120, 120, 255); // grey when no reading

// V geometry. The V is drawn between rows TOP and BOT, with strokes that
// taper toward the centre at BOT.
const TOP: i32 = 4;
const BOT: i32 = 28;
const LEFT_OUTER:  i32 = 4;
const RIGHT_OUTER: i32 = 27;
const STROKE: i32 = 3;
const SPAN: i32 = 11; // outer-x change between TOP and BOT

fn fill_color(pct: u8) -> Rgba {
    if pct < 20      { Rgba(255,  60,  60, 255) }
    else if pct < 60 { Rgba(255, 165,   0, 255) }
    else             { Rgba( 70, 220,  90, 255) }
}

fn put(buf: &mut [u8], x: i32, y: i32, c: Rgba) {
    if x < 0 || y < 0 || (x as u32) >= W || (y as u32) >= H { return; }
    let i = ((y as u32 * W + x as u32) * 4) as usize;
    buf[i]     = c.0;
    buf[i + 1] = c.1;
    buf[i + 2] = c.2;
    buf[i + 3] = c.3;
}

fn fill_row(buf: &mut [u8], y: i32, x_start: i32, x_end: i32, c: Rgba) {
    if x_end < x_start { return; }
    for x in x_start..=x_end { put(buf, x, y, c); }
}

// At row y, what are the outer/inner edges of the V's strokes?
// (left_outer, left_inner, right_inner, right_outer)
fn edges_at(y: i32) -> (i32, i32, i32, i32) {
    let progress = (y - TOP).max(0).min(BOT - TOP);
    let shift = (SPAN * progress) / (BOT - TOP);
    let lo = LEFT_OUTER + shift;
    let li = lo + STROKE;
    let ro = RIGHT_OUTER - shift;
    let ri = ro - STROKE;
    (lo, li, ri, ro)
}

pub fn build_icon_rgba(pct: Option<u8>) -> Vec<u8> {
    let mut buf = vec![0u8; (W * H * 4) as usize];

    // === Fill (drawn first so the outline overlaps it cleanly) ===
    if let Some(p) = pct {
        let fill = fill_color(p);
        // Available fill rows: from TOP+1 down to roughly where the V tip closes (~y=23).
        let fill_top = BOT - 1;
        let fill_bot = TOP + 1;
        // Actually fill from top down: pct=100 -> entire V interior filled.
        // Compute the y at which the fill surface sits.
        // pct=100 -> surface_y = TOP+1 (full)
        // pct=0   -> surface_y = BOT-1 (empty)
        let span = (fill_top - fill_bot) as i32;
        let surface_y = fill_top - (span * p as i32 / 100);

        for y in surface_y..=fill_top {
            let (_lo, li, ri, _ro) = edges_at(y);
            if ri >= li {
                fill_row(&mut buf, y, li, ri, fill);
            }
        }
    } else {
        // Offline: a thin grey line at the bottom interior so the V doesn't look broken.
        let (_lo, li, ri, _ro) = edges_at(BOT - 2);
        if ri >= li {
            fill_row(&mut buf, BOT - 2, li, ri, OFFLINE);
        }
    }

    // === Outline (always green) ===
    for y in TOP..=BOT {
        let (lo, li, ri, ro) = edges_at(y);
        // Left stroke: lo..li
        if li >= lo { fill_row(&mut buf, y, lo, li, OUTLINE); }
        // Right stroke: ri..ro
        if ro >= ri { fill_row(&mut buf, y, ri, ro, OUTLINE); }
    }

    // Cap the bottom point — if the strokes overlap at the tip, draw a small
    // wedge so the V comes to a clean point.
    for y in BOT-1..=BOT+1 {
        let (lo, _li, _ri, ro) = edges_at((y).min(BOT));
        if ro >= lo {
            fill_row(&mut buf, y, lo.max(LEFT_OUTER), ro.min(RIGHT_OUTER), OUTLINE);
        }
    }

    buf
}

pub fn icon_dim() -> (u32, u32) {
    (W, H)
}
