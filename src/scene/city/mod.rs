mod style;

use crate::render::TerminalRenderer;
use crate::scene::art::{
    MountainColors, art_height, art_width, hash, place_outward, render_art_clipped,
    render_art_opaque, render_mountains,
};
use crate::scene::{Scene, SceneContext, SceneLayout};
use std::io;
use std::time::Instant;
use style::CitySceneStyle;

const CAPITOL_ASCII: &str = include_str!("assets/capitol.txt");
const TRAX_ASCII: &str = include_str!("assets/trax.txt");

/// Column of the Capitol's flagpole within its art.
const FLAGPOLE_X_OFFSET: u16 = 21;
/// Tallest the mountains get, in rows above the street.
const MOUNTAIN_MAX_HEIGHT: u16 = 12;
/// A TRAX train passes once per this many seconds...
const TRAIN_PERIOD_SECS: f64 = 45.0;
/// ...moving this many columns per second.
const TRAIN_SPEED: f64 = 30.0;
/// Below this wind speed (m/s, about 1 mph) the flag hangs limp.
const FLAG_CALM_MS: f64 = 0.5;
/// Ripple frame time is this divided by the wind speed in m/s.
const FLAG_SPEED_FACTOR: f64 = 1500.0;
/// Fastest and slowest ripple frame times, in milliseconds.
const FLAG_FRAME_MS_RANGE: (f64, f64) = (100.0, 1000.0);
/// Longest the flag gets in a strong wind, in columns.
const FLAG_MAX_LENGTH: usize = 5;
/// The two ripple frames of a flying flag.
const FLAG_FRAMES: [&str; 2] = ["~=~=~", "=~=~="];
/// At night each window switches on or off about this often, in seconds.
const WINDOW_TOGGLE_SECS: u64 = 30;
/// Window patterns, repeated across each floor of a tower.
const WINDOW_PATTERNS: [&str; 4] = ["[] ", ": ", "## ", "| "];
/// Columns between towers.
const TOWER_GAP: u16 = 1;
/// Towers generated per side; more than any screen can fit.
const TOWERS_PER_SIDE: u16 = 80;

/// A downtown tower, generated from its seed.
#[derive(Clone, Copy)]
struct Tower {
    seed: u16,
    width: u16,
    height: u16,
}

impl Tower {
    /// Builds the `index`th tower out from the Capitol. Towers nearer the
    /// center tend to be taller, giving the skyline its shape.
    fn new(seed: u16, index: u16, max_height: u16) -> Self {
        const MIN_HEIGHT: u16 = 5;
        let max_height = max_height.max(MIN_HEIGHT);
        let height = MIN_HEIGHT + hash(seed, 2) as u16 % (max_height - MIN_HEIGHT + 1);
        let closeness = 1.0 - index.min(10) as f64 * 0.05;
        Self {
            seed,
            width: 7 + hash(seed, 1) as u16 % 8,
            height: ((height as f64 * closeness) as u16).max(MIN_HEIGHT),
        }
    }

    fn width(self) -> u16 {
        self.width
    }
}

/// Salt Lake City: the State Capitol among downtown towers, with the Wasatch
/// behind and TRAX passing on the street. The Capitol's flag flies in the wind.
pub struct CityScene {
    width: u16,
    height: u16,
    started: Instant,
}

impl CityScene {
    /// Street rows: sidewalk, road and lane line, TRAX tracks, and grass.
    const GROUND_HEIGHT: u16 = 7;

    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            started: Instant::now(),
        }
    }

    fn capitol_origin(&self, ground_y: u16) -> (u16, u16) {
        let capitol_x = (self.width / 2).saturating_sub(art_width(CAPITOL_ASCII) / 2);
        let capitol_y = ground_y.saturating_sub(art_height(CAPITOL_ASCII));
        (capitol_x, capitol_y)
    }

    fn render_tower(
        &self,
        renderer: &mut TerminalRenderer,
        tower: Tower,
        x: u16,
        ground_y: u16,
        style: &CitySceneStyle,
    ) -> io::Result<()> {
        let top = ground_y.saturating_sub(tower.height);
        let frame = style.towers[hash(tower.seed, 3) as usize % style.towers.len()];
        let pattern: Vec<char> = WINDOW_PATTERNS
            [hash(tower.seed, 4) as usize % WINDOW_PATTERNS.len()]
        .chars()
        .collect();
        let elapsed = self.started.elapsed();

        // Some towers carry an antenna, with a blinking beacon at night.
        if hash(tower.seed, 5) % 3 == 0 && top >= 2 {
            let antenna_x = x + tower.width / 2;
            let beacon_on = style.is_night && (elapsed.as_millis() / 700) % 2 == 0;
            let (tip, tip_color) = if beacon_on {
                ('*', style.beacon)
            } else {
                ('|', style.antenna)
            };
            renderer.render_char(antenna_x, top - 2, tip, tip_color)?;
            renderer.render_char(antenna_x, top - 1, '|', style.antenna)?;
        }

        for col in 1..tower.width - 1 {
            renderer.render_char(x + col, top, '_', frame)?;
        }

        for y in top + 1..ground_y {
            renderer.render_char(x, y, '|', frame)?;
            renderer.render_char(x + tower.width - 1, y, '|', frame)?;
            for col in 1..tower.width - 1 {
                let cell_x = x + col;
                let (ch, color) = if y == ground_y - 1 {
                    ('_', frame)
                } else {
                    let ch = pattern[(col as usize - 1) % pattern.len()];
                    let color = if !style.is_night {
                        style.tower_glass
                    } else if window_lit(cell_x, y, elapsed.as_secs()) {
                        style.window_lit
                    } else {
                        style.window_dark
                    };
                    (ch, color)
                };
                renderer.render_char(cell_x, y, ch, color)?;
            }
        }

        Ok(())
    }

    fn render_flag(
        &self,
        renderer: &mut TerminalRenderer,
        pole_x: u16,
        top_y: u16,
        wind_speed_ms: f64,
        style: &CitySceneStyle,
    ) -> io::Result<()> {
        renderer.render_char(pole_x, top_y, '|', style.antenna)?;
        renderer.render_char(pole_x, top_y + 1, '|', style.antenna)?;

        match flag_state(wind_speed_ms, self.started.elapsed().as_millis()) {
            None => {
                // Limp: the flag hangs down beside the pole.
                renderer.render_char(pole_x + 1, top_y, ')', style.flag_blue)?;
                renderer.render_char(pole_x + 1, top_y + 1, ')', style.flag_red)?;
            }
            Some((length, frame)) => {
                for (i, ch) in FLAG_FRAMES[frame].chars().take(length).enumerate() {
                    let color = match i {
                        0 => style.flag_blue,
                        _ if i % 2 == 1 => style.flag_red,
                        _ => style.flag_white,
                    };
                    renderer.render_char(pole_x + 1 + i as u16, top_y, ch, color)?;
                }
            }
        }

        Ok(())
    }

    fn render_street(
        &self,
        renderer: &mut TerminalRenderer,
        ground_y: u16,
        style: &CitySceneStyle,
    ) -> io::Result<()> {
        for x in 0..self.width {
            let rows = [
                (if hash(x, 0) < 8 { '.' } else { '_' }, style.sidewalk),
                (if hash(x, 1) < 6 { '.' } else { ' ' }, style.asphalt),
                (if x % 6 < 3 { '-' } else { ' ' }, style.lane),
                (if hash(x, 3) < 6 { '.' } else { ' ' }, style.asphalt),
                ('=', style.rail),
                (if hash(x, 5) < 40 { ',' } else { '^' }, style.grass),
                (if hash(x, 6) < 20 { '^' } else { ' ' }, style.grass),
            ];
            for (row, (ch, color)) in rows.into_iter().enumerate() {
                renderer.render_char(x, ground_y + row as u16, ch, color)?;
            }
        }

        Ok(())
    }

    fn render_train(
        &self,
        renderer: &mut TerminalRenderer,
        ground_y: u16,
        style: &CitySceneStyle,
    ) -> io::Result<()> {
        let train_width = art_width(TRAX_ASCII);
        let x = train_x(self.started.elapsed().as_secs_f64(), train_width);
        if x >= self.width as i32 {
            return Ok(());
        }

        // Wheels sit on the rails, the fifth street row.
        let y = ground_y + 4 - (art_height(TRAX_ASCII) - 1);
        let last_row = art_height(TRAX_ASCII) as usize - 1;
        render_art_clipped(renderer, TRAX_ASCII, x, y, |row, ch| match (row, ch) {
            (r, _) if r == last_row => style.wheel,
            (_, '[' | ']') => style.train_body,
            (_, '_') if row == 1 => style.train_window,
            (r, _) if r == last_row - 1 => style.train_stripe,
            _ => style.train_body,
        })
    }
}

impl Scene for CityScene {
    fn id(&self) -> &'static str {
        "city"
    }

    fn update_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    fn layout(&self) -> SceneLayout {
        SceneLayout {
            ground_y: self.height.saturating_sub(Self::GROUND_HEIGHT),
            chimney_pos: None,
            width: self.width,
            height: self.height,
        }
    }

    fn render(&self, renderer: &mut TerminalRenderer, ctx: &SceneContext<'_>) -> io::Result<()> {
        let ground_y = self.layout().ground_y;
        let (capitol_x, capitol_y) = self.capitol_origin(ground_y);
        let capitol_width = art_width(CAPITOL_ASCII);
        let style = CitySceneStyle::resolve(ctx);

        render_mountains(
            renderer,
            self.width,
            ground_y,
            MOUNTAIN_MAX_HEIGHT.min(ground_y / 2),
            &MountainColors {
                ridge: style.mountain,
                texture: style.mountain_texture,
                snow: style.snow,
            },
        )?;

        // Leave two rows above the tallest tower for an antenna.
        let max_tower_height = (ground_y * 3 / 4).min(ground_y.saturating_sub(3));
        let left: Vec<Tower> = (0..TOWERS_PER_SIDE)
            .map(|i| Tower::new(i, i, max_tower_height))
            .collect();
        let right: Vec<Tower> = (0..TOWERS_PER_SIDE)
            .map(|i| Tower::new(1000 + i, i, max_tower_height))
            .collect();
        let towers = place_outward(
            &left,
            &right,
            Tower::width,
            capitol_x,
            capitol_width,
            self.width,
            TOWER_GAP,
        );
        for (tower, x) in towers {
            self.render_tower(renderer, tower, x, ground_y, &style)?;
        }

        render_art_opaque(
            renderer,
            CAPITOL_ASCII,
            capitol_x,
            capitol_y,
            |_, ch| match ch {
                '[' | ']' => style.capitol_window,
                _ => style.stone,
            },
        )?;
        if capitol_y >= 2 {
            self.render_flag(
                renderer,
                capitol_x + FLAGPOLE_X_OFFSET,
                capitol_y - 2,
                ctx.wind_speed_ms,
                &style,
            )?;
        }

        self.render_street(renderer, ground_y, &style)?;
        self.render_train(renderer, ground_y, &style)?;

        Ok(())
    }
}

/// Whether a lit-window cell is on at night. Each window switches at its own
/// moment, so the city never flips all at once.
fn window_lit(x: u16, y: u16, elapsed_secs: u64) -> bool {
    let phase = hash(x, y.wrapping_add(500)) as u64 * WINDOW_TOGGLE_SECS / 100;
    let period = ((elapsed_secs + phase) / WINDOW_TOGGLE_SECS) as u16;
    hash(x.wrapping_add(period.wrapping_mul(31)), y) < 45
}

/// The flag's length in columns and its ripple frame, or `None` when it's too
/// calm for the flag to fly.
fn flag_state(wind_speed_ms: f64, elapsed_ms: u128) -> Option<(usize, usize)> {
    if wind_speed_ms < FLAG_CALM_MS {
        return None;
    }
    let length = ((wind_speed_ms / 2.0).round() as usize + 2).min(FLAG_MAX_LENGTH);
    let (fastest, slowest) = FLAG_FRAME_MS_RANGE;
    let frame_ms = (FLAG_SPEED_FACTOR / wind_speed_ms).clamp(fastest, slowest) as u128;
    Some((length, (elapsed_ms / frame_ms) as usize % FLAG_FRAMES.len()))
}

/// Left edge of the train at this moment; it enters from off-screen left.
fn train_x(elapsed_secs: f64, train_width: u16) -> i32 {
    ((elapsed_secs % TRAIN_PERIOD_SECS) * TRAIN_SPEED) as i32 - train_width as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flagpole_sits_on_the_cupola() {
        let top_row = CAPITOL_ASCII.lines().next().unwrap();
        let pole = top_row.chars().nth(FLAGPOLE_X_OFFSET as usize);
        assert_eq!(pole, Some('|'));
    }

    #[test]
    fn flag_flies_longer_in_more_wind() {
        assert_eq!(flag_state(0.2, 0), None);
        let (light, _) = flag_state(1.0, 0).unwrap();
        let (strong, _) = flag_state(20.0, 0).unwrap();
        assert!(light < strong);
        assert_eq!(strong, FLAG_MAX_LENGTH);
    }

    #[test]
    fn train_starts_off_screen_and_moves_right() {
        assert_eq!(train_x(0.0, 42), -42);
        assert!(train_x(1.0, 42) > train_x(0.5, 42));
    }

    #[test]
    fn towers_shrink_away_from_center() {
        let near = Tower::new(7, 0, 20);
        let far = Tower::new(7, 10, 20);
        assert!(far.height <= near.height);
        assert!((7..15).contains(&near.width));
    }
}
