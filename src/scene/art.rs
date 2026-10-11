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
    fn place_outward_fills_right_side() {
        let placed = place_outward(&[], &[5u16], |w| w, 0, 10, 30, 2);
        assert_eq!(placed, vec![(5, 12), (5, 19)]);
    }
}
