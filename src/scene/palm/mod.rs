//! A palm tree whose fronds sway with the wind, shared by the sunny scenes.

use crate::render::TerminalRenderer;
use crate::scene::art::{art_height, art_width, render_art_clipped, render_art_opaque};
use crossterm::style::Color;
use std::io;

const PALM_TRUNK_ASCII: &str = include_str!("assets/palm_trunk.txt");
/// Palm crowns: still, a light breeze, and a strong wind.
const PALM_CROWNS: [&str; 3] = [
    include_str!("assets/palm_calm.txt"),
    include_str!("assets/palm_wind_1.txt"),
    include_str!("assets/palm_wind_2.txt"),
];

/// Below this wind speed (m/s, about 2 mph) the palms stand still.
const PALM_CALM_MS: f64 = 1.0;
/// From this wind speed (m/s, about 11 mph) the palms thrash in the wind.
const PALM_STRONG_MS: f64 = 5.0;
/// Sway frame time is this divided by the wind speed in m/s.
const PALM_SPEED_FACTOR: f64 = 2000.0;
/// Fastest and slowest sway frame times, in milliseconds.
const PALM_FRAME_MS_RANGE: (f64, f64) = (150.0, 1200.0);

/// Columns a palm takes up, including its widest crown.
pub fn width() -> u16 {
    PALM_CROWNS
        .iter()
        .map(|crown| art_width(crown))
        .max()
        .unwrap_or(0)
}

/// Draws a palm with its left edge at `x`, standing on the row above `base_y`.
pub fn render_palm(
    renderer: &mut TerminalRenderer,
    x: u16,
    base_y: u16,
    wind_speed_ms: f64,
    elapsed_ms: u128,
    frond: Color,
    trunk: Color,
) -> io::Result<()> {
    let trunk_y = base_y.saturating_sub(art_height(PALM_TRUNK_ASCII));
    render_art_opaque(renderer, PALM_TRUNK_ASCII, x, trunk_y, |_, _| trunk)?;

    let crown = PALM_CROWNS[palm_crown(wind_speed_ms, elapsed_ms)];
    let crown_y = trunk_y.saturating_sub(art_height(crown));
    render_art_clipped(renderer, crown, x as i32, crown_y, |_, ch| match ch {
        '|' => trunk,
        _ => frond,
    })
}

/// Which palm crown to show: still when calm, swaying between the still and
/// breezy crowns in light wind, and thrashing between the two windy crowns in
/// strong wind. Faster wind sways faster.
fn palm_crown(wind_speed_ms: f64, elapsed_ms: u128) -> usize {
    if wind_speed_ms < PALM_CALM_MS {
        return 0;
    }
    let (fastest, slowest) = PALM_FRAME_MS_RANGE;
    let frame_ms = (PALM_SPEED_FACTOR / wind_speed_ms).clamp(fastest, slowest) as u128;
    let swing = (elapsed_ms / frame_ms) as usize % 2;
    if wind_speed_ms < PALM_STRONG_MS {
        swing
    } else {
        1 + swing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palm_crowns_sit_on_the_trunk() {
        let trunk_x = PALM_TRUNK_ASCII.lines().next().unwrap().find(')');
        for crown in PALM_CROWNS {
            let hub = crown.lines().find_map(|line| line.find("\\|/"));
            assert_eq!(hub.map(|x| x + 1), trunk_x);
        }
    }

    #[test]
    fn palms_sway_more_in_stronger_wind() {
        assert_eq!(palm_crown(0.0, 0), 0);
        assert_eq!(palm_crown(0.0, 5000), 0);
        let breezy: Vec<usize> = (0..20).map(|t| palm_crown(3.0, t * 100)).collect();
        assert!(breezy.contains(&0) && breezy.contains(&1) && !breezy.contains(&2));
        let windy: Vec<usize> = (0..20).map(|t| palm_crown(10.0, t * 100)).collect();
        assert!(windy.contains(&1) && windy.contains(&2) && !windy.contains(&0));
    }
}
