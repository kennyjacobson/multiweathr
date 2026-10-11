mod style;

use crate::render::TerminalRenderer;
use crate::scene::art::{
    art_height, art_width, hash, place_outward, render_art_clipped, render_art_opaque, render_water,
};
use crate::scene::{Scene, SceneContext, SceneLayout, palm};
use std::io;
use std::time::Instant;
use style::BeachSceneStyle;

const LIFEGUARD_TOWER_ASCII: &str = include_str!("assets/lifeguard_tower.txt");
const UMBRELLA_ASCII: &str = include_str!("assets/umbrella.txt");
const VOLLEYBALL_NET_ASCII: &str = include_str!("assets/volleyball_net.txt");
const SURFBOARDS_ASCII: &str = include_str!("assets/surfboards.txt");
const PIER_CAFE_ASCII: &str = include_str!("assets/pier_cafe.txt");
const SURFER_ASCII: &str = include_str!("assets/surfer.txt");

/// Rows of open ocean between the horizon and the shore break.
const OCEAN_ROWS: u16 = 5;
/// Rows of sand at the bottom of the screen.
const SAND_ROWS: u16 = 7;
/// The pier reaches this far across the screen, in percent of its width.
const PIER_REACH_PERCENT: u16 = 45;
/// Columns between the pier's pilings...
const PILING_SPACING: u16 = 5;
/// ...and between its lamp posts.
const LAMP_SPACING: u16 = 12;
/// Swells drift this many columns per second on a calm day...
const SWELL_DRIFT_CALM: f64 = 0.6;
/// ...plus this many per m/s of wind...
const SWELL_DRIFT_PER_MS: f64 = 0.5;
/// ...up to this limit.
const SWELL_DRIFT_MAX: f64 = 8.0;
/// The shore break churns to a new foam pattern this often, in milliseconds.
const FOAM_FRAME_MS: u128 = 400;
/// A surfer rides by once per this many seconds...
const SURFER_PERIOD_SECS: f64 = 30.0;
/// ...moving this many columns per second.
const SURFER_SPEED: f64 = 9.0;
/// Empty columns between props.
const PROP_GAP: u16 = 3;

/// Something on the sand beside the lifeguard tower.
#[derive(Clone, Copy)]
enum Prop {
    Umbrella,
    Palm,
    Surfboards,
    VolleyballNet,
}

// Props are placed outward from the tower, cycling until the screen is full.
// Palms only go on the right: on the left their crowns would tangle with the pier.
const LEFT_PROPS: [Prop; 3] = [Prop::Umbrella, Prop::Surfboards, Prop::Umbrella];
const RIGHT_PROPS: [Prop; 6] = [
    Prop::VolleyballNet,
    Prop::Palm,
    Prop::Umbrella,
    Prop::Surfboards,
    Prop::Palm,
    Prop::Umbrella,
];

impl Prop {
    fn width(self) -> u16 {
        match self {
            Prop::Umbrella => art_width(UMBRELLA_ASCII),
            Prop::Palm => palm::width(),
            Prop::Surfboards => art_width(SURFBOARDS_ASCII),
            Prop::VolleyballNet => art_width(VOLLEYBALL_NET_ASCII),
        }
    }
}

/// Huntington Beach, "Surf City USA": the pier with its lamps and cafe, the
/// Pacific rolling in, a surfer, and a lifeguard tower among umbrellas and
/// palms. Swells and palms move with the wind.
pub struct BeachScene {
    width: u16,
    height: u16,
    started: Instant,
}

impl BeachScene {
    /// Ocean, the shore break, and the sand.
    const GROUND_HEIGHT: u16 = OCEAN_ROWS + 1 + SAND_ROWS;

    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            started: Instant::now(),
        }
    }

    /// Row that things on the sand stand on top of.
    fn sand_base(&self) -> u16 {
        self.height.saturating_sub(1)
    }

    fn render_sea_and_sand(
        &self,
        renderer: &mut TerminalRenderer,
        horizon_y: u16,
        wind_speed_ms: f64,
        style: &BeachSceneStyle,
    ) -> io::Result<()> {
        let elapsed = self.started.elapsed();
        let drift = (SWELL_DRIFT_CALM + wind_speed_ms * SWELL_DRIFT_PER_MS).min(SWELL_DRIFT_MAX);
        render_water(
            renderer,
            self.width,
            horizon_y,
            OCEAN_ROWS,
            elapsed.as_secs_f64() * drift,
            &style.ocean,
            7,
        )?;

        // The shore break churns between foam patterns.
        let foam_y = horizon_y + OCEAN_ROWS;
        let churn = (elapsed.as_millis() / FOAM_FRAME_MS) as u16;
        for x in 0..self.width {
            let r = hash(x.wrapping_add(churn.wrapping_mul(17)), foam_y);
            let ch = match r {
                0..=34 => '~',
                35..=54 => '^',
                55..=69 => '-',
                _ => ' ',
            };
            renderer.render_char(x, foam_y, ch, style.foam[r as usize % 2])?;
        }

        for row in 0..SAND_ROWS {
            let y = foam_y + 1 + row;
            for x in 0..self.width {
                let (ch, color) = match (row, hash(x, y)) {
                    (0, r) if r < 30 => ('_', style.wet_sand),
                    (0, _) => ('.', style.wet_sand),
                    (_, 0..=9) => ('.', style.sand_texture),
                    (_, 10..=13) => (':', style.sand_texture),
                    _ => (' ', style.sand),
                };
                renderer.render_char(x, y, ch, color)?;
            }
        }

        Ok(())
    }

    fn render_pier(
        &self,
        renderer: &mut TerminalRenderer,
        horizon_y: u16,
        style: &BeachSceneStyle,
    ) -> io::Result<()> {
        let Some(deck_y) = horizon_y.checked_sub(1) else {
            return Ok(());
        };
        let pier_end = self.width * PIER_REACH_PERCENT / 100;
        let cafe_width = art_width(PIER_CAFE_ASCII);
        let cafe_x = pier_end.saturating_sub(cafe_width);

        for x in 0..pier_end {
            renderer.render_char(x, deck_y, '=', style.pier_wood)?;
            if x % PILING_SPACING == 2 {
                for y in horizon_y..horizon_y + OCEAN_ROWS {
                    renderer.render_char(x, y, '|', style.piling)?;
                }
            }
            if deck_y >= 2 && x + 1 < cafe_x {
                let rail = if x % 4 == 0 { '+' } else { '-' };
                renderer.render_char(x, deck_y - 1, rail, style.pier_wood)?;
                if x % LAMP_SPACING == 6 {
                    renderer.render_char(x, deck_y - 1, '|', style.piling)?;
                    renderer.render_char(x, deck_y - 2, 'o', style.lamp)?;
                }
            }
        }

        let cafe_y = deck_y.saturating_sub(art_height(PIER_CAFE_ASCII));
        render_art_opaque(
            renderer,
            PIER_CAFE_ASCII,
            cafe_x,
            cafe_y,
            |row, ch| match ch {
                _ if row < 2 => style.cafe_roof,
                '[' | ']' => style.window,
                _ => style.cafe_wall,
            },
        )
    }

    fn render_surfer(
        &self,
        renderer: &mut TerminalRenderer,
        horizon_y: u16,
        style: &BeachSceneStyle,
    ) -> io::Result<()> {
        let elapsed = self.started.elapsed().as_secs_f64();
        let surfer_width = art_width(SURFER_ASCII) as i32;
        let x = ((elapsed % SURFER_PERIOD_SECS) * SURFER_SPEED) as i32 - surfer_width;
        if x >= self.width as i32 {
            return Ok(());
        }

        // Riding the fourth ocean row, just outside the shore break.
        let board_y = horizon_y + 3;
        let y = board_y + 1 - art_height(SURFER_ASCII);
        let board_row = art_height(SURFER_ASCII) as usize - 1;
        render_art_clipped(renderer, SURFER_ASCII, x, y, |row, _| {
            if row == board_row {
                style.surfboard
            } else {
                style.surfer
            }
        })?;

        // White water trailing behind the board.
        for i in 1..=6 {
            let col = x - i;
            if col >= 0 {
                let ch = if i % 2 == 0 { '~' } else { '^' };
                renderer.render_char(col as u16, board_y, ch, style.foam[0])?;
            }
        }

        Ok(())
    }

    fn render_prop(
        &self,
        renderer: &mut TerminalRenderer,
        prop: Prop,
        x: u16,
        wind_speed_ms: f64,
        style: &BeachSceneStyle,
    ) -> io::Result<()> {
        let base = self.sand_base();
        match prop {
            Prop::Palm => palm::render_palm(
                renderer,
                x,
                base,
                wind_speed_ms,
                self.started.elapsed().as_millis(),
                style.frond,
                style.palm_trunk,
            ),
            Prop::Umbrella => {
                let y = base.saturating_sub(art_height(UMBRELLA_ASCII));
                // Stripes come from the slant of each panel.
                let [first, second, third] = style.umbrella;
                render_art_opaque(renderer, UMBRELLA_ASCII, x, y, |row, ch| match (row, ch) {
                    (3.., _) => style.pole,
                    (_, '/' | '\'') => first,
                    (_, '\\' | '-') => second,
                    _ => third,
                })
            }
            Prop::Surfboards => {
                let y = base.saturating_sub(art_height(SURFBOARDS_ASCII));
                let board_width = 5;
                for (i, line) in SURFBOARDS_ASCII.lines().enumerate() {
                    for (j, ch) in line.chars().enumerate() {
                        if ch != ' ' {
                            let color = style.boards[(j / board_width) % style.boards.len()];
                            renderer.render_char(x + j as u16, y + i as u16, ch, color)?;
                        }
                    }
                }
                Ok(())
            }
            Prop::VolleyballNet => {
                let y = base.saturating_sub(art_height(VOLLEYBALL_NET_ASCII));
                render_art_opaque(renderer, VOLLEYBALL_NET_ASCII, x, y, |_, ch| match ch {
                    'o' => style.ball,
                    '+' | '-' => style.net,
                    _ => style.pole,
                })
            }
        }
    }
}

impl Scene for BeachScene {
    fn id(&self) -> &'static str {
        "beach"
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
        let horizon_y = self.layout().ground_y;
        let style = BeachSceneStyle::resolve(ctx);

        self.render_sea_and_sand(renderer, horizon_y, ctx.wind_speed_ms, &style)?;
        self.render_pier(renderer, horizon_y, &style)?;
        self.render_surfer(renderer, horizon_y, &style)?;

        let tower_width = art_width(LIFEGUARD_TOWER_ASCII);
        let tower_x = (self.width / 2).saturating_sub(tower_width / 2);
        let tower_y = self
            .sand_base()
            .saturating_sub(art_height(LIFEGUARD_TOWER_ASCII));
        render_art_opaque(
            renderer,
            LIFEGUARD_TOWER_ASCII,
            tower_x,
            tower_y,
            |row, ch| match ch {
                _ if row < 2 => style.tower_roof,
                '#' => style.tower_glass,
                '6' => style.tower_number,
                '=' | '.' | '\\' => style.pier_wood,
                _ if row >= 5 => style.tower_legs,
                _ => style.tower_body,
            },
        )?;

        let props = place_outward(
            &LEFT_PROPS,
            &RIGHT_PROPS,
            Prop::width,
            tower_x,
            tower_width,
            self.width,
            PROP_GAP,
        );
        for (prop, x) in props {
            self.render_prop(renderer, prop, x, ctx.wind_speed_ms, &style)?;
        }

        Ok(())
    }
}

// weathr refuses to run below 20 rows; leave sky above the ocean.
const _: () = assert!(BeachScene::GROUND_HEIGHT < 20 - 4);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surfer_stands_on_the_board() {
        let rows: Vec<&str> = SURFER_ASCII.lines().collect();
        assert!(rows.last().unwrap().trim().chars().all(|ch| ch == '='));
    }
}
