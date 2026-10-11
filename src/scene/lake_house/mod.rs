mod style;

use crate::render::TerminalRenderer;
use crate::scene::art::{
    MountainColors, art_height, art_width, hash, place_outward, render_art_clipped,
    render_art_opaque, render_mountains, render_water,
};
use crate::scene::{ChimneyPosition, Scene, SceneContext, SceneLayout};
use std::io;
use std::sync::Mutex;
use std::time::Instant;
use style::LakeHouseSceneStyle;

const CABIN_ASCII: &str = include_str!("assets/a_frame.txt");
const PINE_TALL_ASCII: &str = include_str!("assets/pine_tall.txt");
const PINE_SHORT_ASCII: &str = include_str!("assets/pine_short.txt");
const SHAKE_SIGN_ASCII: &str = include_str!("assets/shake_sign.txt");
const SAILBOAT_ASCII: &str = include_str!("assets/sailboat.txt");

/// Rows of shoreline (grass, then sand) between the cabin and the water.
const SHORE_ROWS: u16 = 2;
/// Tallest the mountains get, in rows above the shoreline.
const MOUNTAIN_MAX_HEIGHT: u16 = 14;
/// Waves drift this many columns per second on a calm day...
const WAVE_DRIFT_CALM: f64 = 0.3;
/// ...plus this many per m/s of wind...
const WAVE_DRIFT_PER_MS: f64 = 0.6;
/// ...up to this limit.
const WAVE_DRIFT_MAX: f64 = 10.0;
/// The sailboat moves this many columns per second per m/s of wind.
const BOAT_SPEED_PER_MS: f64 = 0.4;
/// Empty columns between props.
const PROP_GAP: u16 = 2;

/// Something standing on the shore beside the cabin.
#[derive(Clone, Copy)]
enum Prop {
    PineTall,
    PineShort,
    ShakeSign,
}

// Props are placed outward from the cabin, cycling until the screen is full.
const LEFT_PROPS: [Prop; 5] = [
    Prop::PineTall,
    Prop::ShakeSign,
    Prop::PineShort,
    Prop::PineTall,
    Prop::PineShort,
];
const RIGHT_PROPS: [Prop; 4] = [
    Prop::PineShort,
    Prop::PineTall,
    Prop::PineTall,
    Prop::PineShort,
];

impl Prop {
    fn ascii(self) -> &'static str {
        match self {
            Prop::PineTall => PINE_TALL_ASCII,
            Prop::PineShort => PINE_SHORT_ASCII,
            Prop::ShakeSign => SHAKE_SIGN_ASCII,
        }
    }

    fn width(self) -> u16 {
        art_width(self.ascii())
    }
}

/// Where the sailboat is and when it last moved.
struct Boat {
    x: f64,
    moved_at: Instant,
}

/// Bear Lake, Utah: an A-frame cabin in the pines on a turquoise lake, with
/// mountains behind. Waves and a sailboat move with the wind.
pub struct LakeHouseScene {
    width: u16,
    height: u16,
    started: Instant,
    boat: Mutex<Boat>,
}

impl LakeHouseScene {
    /// Shoreline plus water rows below the cabin.
    const GROUND_HEIGHT: u16 = 7;
    const CHIMNEY_X_OFFSET: u16 = 20;
    const CHIMNEY_Y_OFFSET: u16 = 2;

    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            started: Instant::now(),
            boat: Mutex::new(Boat {
                x: width as f64 * 0.7,
                moved_at: Instant::now(),
            }),
        }
    }

    fn cabin_origin(&self, ground_y: u16) -> (u16, u16) {
        let cabin_x = (self.width / 2).saturating_sub(art_width(CABIN_ASCII) / 2);
        let cabin_y = ground_y.saturating_sub(art_height(CABIN_ASCII));
        (cabin_x, cabin_y)
    }

    fn render_shore_and_water(
        &self,
        renderer: &mut TerminalRenderer,
        ground_y: u16,
        wind_speed_ms: f64,
        style: &LakeHouseSceneStyle,
    ) -> io::Result<()> {
        for x in 0..self.width {
            let r = hash(x, 0);
            let (ch, color) = match r {
                0..=29 => ('^', style.shore_grass),
                30..=49 => (',', style.shore_grass),
                _ => ('.', style.sand),
            };
            renderer.render_char(x, ground_y, ch, color)?;

            let ch = match hash(x, 1) {
                0..=24 => '.',
                25..=34 => ':',
                _ => '_',
            };
            renderer.render_char(x, ground_y + 1, ch, style.sand)?;
        }

        render_water(
            renderer,
            self.width,
            ground_y + SHORE_ROWS,
            Self::GROUND_HEIGHT - SHORE_ROWS,
            self.started.elapsed().as_secs_f64() * wave_drift(wind_speed_ms),
            &style.water,
            0,
        )
    }

    fn render_boat(
        &self,
        renderer: &mut TerminalRenderer,
        ground_y: u16,
        wind_speed_ms: f64,
        style: &LakeHouseSceneStyle,
    ) -> io::Result<()> {
        let boat_width = art_width(SAILBOAT_ASCII) as f64;
        let x = {
            let mut boat = self.boat.lock().unwrap_or_else(|e| e.into_inner());
            let dt = boat.moved_at.elapsed().as_secs_f64();
            boat.moved_at = Instant::now();
            boat.x += dt * wind_speed_ms * BOAT_SPEED_PER_MS;
            // Sail off the right edge and come back in from the left.
            if boat.x > self.width as f64 {
                boat.x = -boat_width;
            }
            boat.x.round() as i32
        };

        // Hull sits on the third water row.
        let y = ground_y + SHORE_ROWS + 3 - (art_height(SAILBOAT_ASCII) - 1);
        render_art_clipped(renderer, SAILBOAT_ASCII, x, y, |row, ch| match (row, ch) {
            (3, _) => style.hull,
            (_, '|') => style.trim,
            _ => style.sail,
        })
    }

    fn render_prop(
        &self,
        renderer: &mut TerminalRenderer,
        prop: Prop,
        x: u16,
        ground_y: u16,
        style: &LakeHouseSceneStyle,
    ) -> io::Result<()> {
        let ascii = prop.ascii();
        let y = ground_y.saturating_sub(art_height(ascii));
        let last_row = art_height(ascii) as usize - 1;
        match prop {
            Prop::PineTall | Prop::PineShort => {
                render_art_opaque(renderer, ascii, x, y, |row, _| {
                    if row == last_row {
                        style.trunk
                    } else {
                        style.pine
                    }
                })
            }
            Prop::ShakeSign => render_art_opaque(renderer, ascii, x, y, |row, ch| {
                if row == last_row {
                    style.trunk
                } else if ch.is_ascii_alphabetic() {
                    style.sign_text
                } else {
                    style.sign_board
                }
            }),
        }
    }
}

impl Scene for LakeHouseScene {
    fn id(&self) -> &'static str {
        "lake-house"
    }

    fn update_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    fn layout(&self) -> SceneLayout {
        let ground_y = self.height.saturating_sub(Self::GROUND_HEIGHT);
        let (cabin_x, cabin_y) = self.cabin_origin(ground_y);

        SceneLayout {
            ground_y,
            chimney_pos: Some(ChimneyPosition {
                x: cabin_x + Self::CHIMNEY_X_OFFSET,
                y: cabin_y + Self::CHIMNEY_Y_OFFSET,
            }),
            width: self.width,
            height: self.height,
        }
    }

    fn render(&self, renderer: &mut TerminalRenderer, ctx: &SceneContext<'_>) -> io::Result<()> {
        let layout = self.layout();
        let ground_y = layout.ground_y;
        let (cabin_x, cabin_y) = self.cabin_origin(ground_y);
        let style = LakeHouseSceneStyle::resolve(ctx);

        // Leave the upper part of the sky clear for the sun, moon and clouds.
        let mountain_height = MOUNTAIN_MAX_HEIGHT.min(ground_y * 11 / 20);
        render_mountains(
            renderer,
            self.width,
            ground_y,
            mountain_height,
            &MountainColors {
                ridge: style.mountain,
                texture: style.mountain_texture,
                snow: style.snow,
            },
        )?;
        self.render_shore_and_water(renderer, ground_y, ctx.wind_speed_ms, &style)?;

        render_art_opaque(
            renderer,
            CABIN_ASCII,
            cabin_x,
            cabin_y,
            |row, ch| match ch {
                '[' | ']' if row < 4 => style.stovepipe,
                '|' if (2..4).contains(&row) => style.stovepipe,
                '#' => style.glass,
                '[' | ']' => style.trim,
                '/' | '\\' => style.roof,
                _ => style.wood,
            },
        )?;

        let props = place_outward(
            &LEFT_PROPS,
            &RIGHT_PROPS,
            Prop::width,
            cabin_x,
            art_width(CABIN_ASCII),
            self.width,
            PROP_GAP,
        );
        for (prop, x) in props {
            self.render_prop(renderer, prop, x, ground_y, &style)?;
        }

        self.render_boat(renderer, ground_y, ctx.wind_speed_ms, &style)?;

        Ok(())
    }
}

/// How many columns per second the waves drift at this wind speed.
fn wave_drift(wind_speed_ms: f64) -> f64 {
    (WAVE_DRIFT_CALM + wind_speed_ms * WAVE_DRIFT_PER_MS).min(WAVE_DRIFT_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stovepipe_sits_on_the_roof() {
        let row = CABIN_ASCII
            .lines()
            .nth(LakeHouseScene::CHIMNEY_Y_OFFSET as usize)
            .unwrap();
        let pipe = row.chars().nth(LakeHouseScene::CHIMNEY_X_OFFSET as usize);
        assert_eq!(pipe, Some('['));
    }

    #[test]
    fn waves_speed_up_with_wind() {
        assert_eq!(wave_drift(0.0), WAVE_DRIFT_CALM);
        assert!(wave_drift(5.0) > wave_drift(1.0));
        assert_eq!(wave_drift(100.0), WAVE_DRIFT_MAX);
    }
}
