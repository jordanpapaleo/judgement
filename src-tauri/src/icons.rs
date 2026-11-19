//! Menu-bar icons, drawn at runtime so each state differs in shape, not just
//! color: a dot (active), two bars (paused), a triangle (store unavailable).

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
            TrayIcon::Active => ([0, 0, 0], |x, y| {
                (x - 22.0).powi(2) + (y - 22.0).powi(2) <= 9.0_f32.powi(2)
            }),
            TrayIcon::Paused => ([0, 0, 0], |x, y| {
                (12.0..36.0).contains(&y)
                    && ((13.0..19.0).contains(&x) || (25.0..31.0).contains(&x))
            }),
            TrayIcon::Unavailable => ([217, 92, 38], |x, y| {
                // Upward triangle with apex (22, 8) and base y = 37 from x = 5 to 39.
                (8.0..=37.0).contains(&y) && (x - 22.0).abs() <= (y - 8.0) * 17.0 / 29.0
            }),
        };
        Image::new_owned(render(rgb, inside), SIZE, SIZE)
    }
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
