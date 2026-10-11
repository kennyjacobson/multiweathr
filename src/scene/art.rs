//! Helpers shared by scenes that are drawn from ASCII art assets.

use crate::render::TerminalRenderer;
use crossterm::style::Color;
use std::io;

pub fn art_width(ascii: &str) -> u16 {
    ascii
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as u16
}

pub fn art_height(ascii: &str) -> u16 {
    ascii.lines().count() as u16
}

/// Draws ASCII art with its top-left corner at (x, y), leaving spaces transparent.
/// `color` picks a color from the row within the art and the character.
pub fn render_art(
    renderer: &mut TerminalRenderer,
    ascii: &str,
    x: u16,
    y: u16,
    color: impl Fn(usize, char) -> Color,
) -> io::Result<()> {
    for (i, line) in ascii.lines().enumerate() {
        for (j, ch) in line.chars().enumerate() {
            if ch != ' ' {
                renderer.render_char(x + j as u16, y + i as u16, ch, color(i, ch))?;
            }
        }
    }
    Ok(())
}

/// Like [`render_art`], but spaces between a row's first and last character are
/// drawn as blanks, hiding whatever was behind the art (such as mountains).
pub fn render_art_opaque(
    renderer: &mut TerminalRenderer,
    ascii: &str,
    x: u16,
    y: u16,
    color: impl Fn(usize, char) -> Color,
) -> io::Result<()> {
    for (i, line) in ascii.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let Some(first) = chars.iter().position(|&ch| ch != ' ') else {
            continue;
        };
        let last = chars.iter().rposition(|&ch| ch != ' ').unwrap_or(first);
        for (j, &ch) in chars.iter().enumerate().take(last + 1).skip(first) {
            renderer.render_char(x + j as u16, y + i as u16, ch, color(i, ch))?;
        }
    }
    Ok(())
}

/// Like [`render_art_opaque`], but `x` may be negative so art can slide in
/// from off the left edge; columns left of the screen are skipped.
pub fn render_art_clipped(
    renderer: &mut TerminalRenderer,
    ascii: &str,
    x: i32,
    y: u16,
    color: impl Fn(usize, char) -> Color,
) -> io::Result<()> {
    for (i, line) in ascii.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let Some(first) = chars.iter().position(|&ch| ch != ' ') else {
            continue;
        };
        let last = chars.iter().rposition(|&ch| ch != ' ').unwrap_or(first);
        for (j, &ch) in chars.iter().enumerate().take(last + 1).skip(first) {
            let col = x + j as i32;
            if (0..=u16::MAX as i32).contains(&col) {
                renderer.render_char(col as u16, y + i as u16, ch, color(i, ch))?;
            }
        }
    }
    Ok(())
}

/// Lays props out from a centerpiece toward each screen edge, returning each
/// prop with its x. Each side cycles through its list; a prop that doesn't fit
/// is skipped for the next one, and a side stops once none fit.
pub fn place_outward<P: Copy>(
    left: &[P],
    right: &[P],
    width_of: impl Fn(P) -> u16,
    center_x: u16,
    center_width: u16,
    screen_width: u16,
    gap: u16,
) -> Vec<(P, u16)> {
    let mut placed = Vec::new();

    let mut edge = center_x;
    let mut misses = 0;
    for &prop in left.iter().cycle() {
        if misses == left.len() {
            break;
        }
        let needed = gap + width_of(prop);
        if edge > needed {
            edge -= needed;
            placed.push((prop, edge));
            misses = 0;
        } else {
            misses += 1;
        }
    }

    let mut edge = center_x + center_width;
    let mut misses = 0;
    for &prop in right.iter().cycle() {
        if misses == right.len() {
            break;
        }
        let x = edge + gap;
        if x + width_of(prop) < screen_width {
            placed.push((prop, x));
            edge = x + width_of(prop);
            misses = 0;
        } else {
            misses += 1;
        }
    }

    placed
}

/// Average columns between mountain peaks.
const PEAK_SPACING: u16 = 18;

/// Colors for [`render_mountains`].
pub struct MountainColors {
    pub ridge: Color,
    pub texture: Color,
    pub snow: Color,
}

/// Draws a mountain range standing on `ground_y`, up to `max_height` rows tall,
/// with snow on the highest peaks. The slopes are filled so stars and clouds
/// don't show through them.
pub fn render_mountains(
    renderer: &mut TerminalRenderer,
    screen_width: u16,
    ground_y: u16,
    max_height: u16,
    colors: &MountainColors,
) -> io::Result<()> {
    if max_height == 0 {
        return Ok(());
    }

    for x in 0..screen_width {
        let height = ridge_height(x, max_height);
        let left = ridge_height(x.saturating_sub(1), max_height);
        let right = ridge_height(x + 1, max_height);
        let top = ground_y.saturating_sub(height);

        // Peaks are two columns wide, so their tops draw as `/\`.
        let ridge = if right > height || (left < height && right == height) {
            '/'
        } else if left > height || (right < height && left == height) {
            '\\'
        } else {
            '_'
        };
        let snowy = height + 2 >= max_height;
        let ridge_color = if snowy { colors.snow } else { colors.ridge };
        renderer.render_char(x, top, ridge, ridge_color)?;

        for y in top + 1..ground_y {
            let (ch, color) = if snowy && y <= top + 2 {
                ('.', colors.snow)
            } else if hash(x, y) < 10 {
                ('.', colors.texture)
            } else {
                (' ', colors.ridge)
            };
            renderer.render_char(x, y, ch, color)?;
        }
    }

    Ok(())
}

/// Height of the mountain ridge at column `x`, from 1 to `max`.
/// Peaks sit roughly every `PEAK_SPACING` columns with varied heights, and
/// each slope drops one row per column so it draws as a clean `/` or `\`.
fn ridge_height(x: u16, max: u16) -> u16 {
    let max = max as i32;
    let slot = x / PEAK_SPACING;
    let mut height = 1;
    for s in slot.saturating_sub(2)..=slot + 2 {
        let peak_x = (s * PEAK_SPACING) as i32 + (hash(s, 7) % PEAK_SPACING as u32) as i32;
        let peak_height = max / 2 + hash(s, 9) as i32 % (max - max / 2 + 1);
        // Distance from the two-column peak at peak_x and peak_x + 1.
        let distance = if (x as i32) <= peak_x {
            peak_x - x as i32
        } else {
            x as i32 - peak_x - 1
        };
        height = height.max(peak_height - distance);
    }
    height.clamp(1, max) as u16
}

/// Cheap, stable per-cell noise in 0..100 so textures don't flicker.
pub fn hash(x: u16, y: u16) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    (h ^ (h >> 16)) % 100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_size_uses_widest_line() {
        assert_eq!(art_width(" /\\\n/__\\\n"), 4);
        assert_eq!(art_height(" /\\\n/__\\\n"), 2);
    }

    #[test]
    fn place_outward_skips_props_that_dont_fit() {
        // 10 columns free on the left: the 8-wide prop won't fit after the gap
        // of 3, but the 4-wide one will.
        let placed = place_outward(&[8u16, 4], &[], |w| w, 10, 20, 30, 3);
        assert_eq!(placed, vec![(4, 3)]);
    }

    #[test]
    fn ridge_stays_in_bounds() {
        for x in 0..1000 {
            let height = ridge_height(x, 8);
            assert!((1..=8).contains(&height));
        }
    }

    #[test]
    fn place_outward_fills_right_side() {
        let placed = place_outward(&[], &[5u16], |w| w, 0, 10, 30, 2);
        assert_eq!(placed, vec![(5, 12), (5, 19)]);
    }
}
