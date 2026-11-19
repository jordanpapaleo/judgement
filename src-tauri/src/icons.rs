//! Menu-bar icons, drawn at runtime so each state differs in shape, not just
//! color: the app icon's scales (active), the scales with a pause badge
//! (paused), and an orange triangle (store unavailable).

use tauri::image::Image;

/// 22pt at 2x.
const SIZE: u32 = 44;
const SUPERSAMPLE: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIcon {
    Active,
    Paused,
    Unavailable,
}

impl TrayIcon {
    /// Active and paused are template images (macOS tints them for the menu
    /// bar); unavailable is drawn in color so it never looks like active.
    pub fn is_template(self) -> bool {
        self != TrayIcon::Unavailable
    }

    pub fn image(self) -> Image<'static> {
        let (rgb, inside): ([u8; 3], fn(f32, f32) -> bool) = match self {
            TrayIcon::Active => ([0, 0, 0], scales),
            TrayIcon::Paused => ([0, 0, 0], paused),
            TrayIcon::Unavailable => ([217, 92, 38], |x, y| {
                // Upward triangle with apex (22, 8) and base y = 37 from x = 5 to 39.
                (8.0..=37.0).contains(&y) && (x - 22.0).abs() <= (y - 8.0) * 17.0 / 29.0
            }),
        };
        Image::new_owned(render(rgb, inside), SIZE, SIZE)
    }
}

// Shapes on the 44×44 canvas, simplified from the app icon's geometric scales.

fn rect(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
}

/// A line from (ax, ay) to (bx, by), `w` wide.
fn segment(x: f32, y: f32, (ax, ay): (f32, f32), (bx, by): (f32, f32), w: f32) -> bool {
    let (dx, dy) = (bx - ax, by - ay);
    let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    let (cx, cy) = (ax + t * dx, ay + t * dy);
    (x - cx).powi(2) + (y - cy).powi(2) <= (w / 2.0).powi(2)
}

/// A trapezoid whose top edge at `y0` spans `top` and bottom edge at `y1` spans `bottom`.
fn trapezoid(x: f32, y: f32, y0: f32, y1: f32, top: (f32, f32), bottom: (f32, f32)) -> bool {
    if !(y0..=y1).contains(&y) {
        return false;
    }
    let t = (y - y0) / (y1 - y0);
    let left = top.0 + (bottom.0 - top.0) * t;
    let right = top.1 + (bottom.1 - top.1) * t;
    (left..=right).contains(&x)
}

fn scales(x: f32, y: f32) -> bool {
    const STRING: f32 = 1.6;
    rect(x, y, 20.6, 6.0, 23.4, 36.0) // post
        || rect(x, y, 18.5, 4.5, 25.5, 7.5) // cap
        || rect(x, y, 5.5, 9.5, 38.5, 12.3) // beam
        || segment(x, y, (9.0, 11.0), (4.6, 25.0), STRING)
        || segment(x, y, (9.0, 11.0), (13.4, 25.0), STRING)
        || segment(x, y, (35.0, 11.0), (30.6, 25.0), STRING)
        || segment(x, y, (35.0, 11.0), (39.4, 25.0), STRING)
        || trapezoid(x, y, 24.5, 29.5, (2.5, 15.5), (5.5, 12.5)) // left pan
        || trapezoid(x, y, 24.5, 29.5, (28.5, 41.5), (31.5, 38.5)) // right pan
        || trapezoid(x, y, 34.5, 39.5, (16.5, 27.5), (12.5, 31.5)) // base
}

/// The scales with a pause badge cut into the lower-right corner.
fn paused(x: f32, y: f32) -> bool {
    let badge = (x - 35.0).powi(2) + (y - 34.5).powi(2) <= 9.5_f32.powi(2);
    let bars = rect(x, y, 31.6, 30.0, 33.8, 39.0) || rect(x, y, 36.2, 30.0, 38.4, 39.0);
    (scales(x, y) && !badge) || bars
}

fn render(rgb: [u8; 3], inside: fn(f32, f32) -> bool) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let samples = (SUPERSAMPLE * SUPERSAMPLE) as f32;
    for py in 0..SIZE {
        for px in 0..SIZE {
            let mut hits = 0;
            for sy in 0..SUPERSAMPLE {
                for sx in 0..SUPERSAMPLE {
                    let x = px as f32 + (sx as f32 + 0.5) / SUPERSAMPLE as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / SUPERSAMPLE as f32;
                    if inside(x, y) {
                        hits += 1;
                    }
                }
            }
            let alpha = (hits as f32 / samples * 255.0).round() as u8;
            rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], alpha]);
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coverage(icon: TrayIcon) -> Vec<bool> {
        icon.image().rgba().chunks(4).map(|p| p[3] > 127).collect()
    }

    #[test]
    fn states_differ_in_shape() {
        let (a, p, u) = (
            coverage(TrayIcon::Active),
            coverage(TrayIcon::Paused),
            coverage(TrayIcon::Unavailable),
        );
        assert_ne!(a, p);
        assert_ne!(a, u);
        assert_ne!(p, u);
        assert!(!TrayIcon::Unavailable.is_template());
    }
}
