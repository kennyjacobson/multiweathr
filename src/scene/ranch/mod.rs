mod style;

use crate::render::TerminalRenderer;
use crate::scene::art::{art_height, art_width, hash, place_outward, render_art};
use crate::scene::{ChimneyPosition, Scene, SceneContext, SceneLayout};
use std::io;
use std::ops::Range;
use std::time::Instant;
use style::RanchSceneStyle;

const HOUSE_ASCII: &str = include_str!("assets/ranch_house.txt");
const LONGHORN_ASCII: &str = include_str!("assets/longhorn.txt");
const PRICKLY_PEAR_ASCII: &str = include_str!("assets/prickly_pear.txt");
const WINDMILL_WHEEL_FRAMES: [&str; 2] = [
    include_str!("assets/windmill_wheel_1.txt"),
    include_str!("assets/windmill_wheel_2.txt"),
];
const WINDMILL_TOWER_ASCII: &str = include_str!("assets/windmill_tower.txt");
const STOCK_TANK_ASCII: &str = include_str!("assets/stock_tank.txt");
const FENCE_ASCII: &str = include_str!("assets/fence.txt");

/// Below this wind speed (m/s, about 1 mph) the windmill stands still.
const WINDMILL_CALM_MS: f64 = 0.5;
/// Frame time is this divided by the wind speed in m/s, so 5 m/s (11 mph)
/// shows each wheel frame for 240ms.
const WINDMILL_SPEED_FACTOR: f64 = 1200.0;
/// Fastest and slowest frame times, in milliseconds.
const WINDMILL_FRAME_MS_RANGE: (f64, f64) = (100.0, 1000.0);
/// Empty columns between props.
const PROP_GAP: u16 = 3;

/// Something standing on the ground beside the house.
#[derive(Clone, Copy)]
enum Prop {
    PricklyPear,
    Windmill,
    StockTank,
    Longhorn,
    Fence,
}

// Props are placed outward from the house, cycling until the screen is full.
const LEFT_PROPS: [Prop; 6] = [
    Prop::Windmill,
    Prop::StockTank,
    Prop::PricklyPear,
    Prop::Fence,
    Prop::Longhorn,
    Prop::PricklyPear,
];
const RIGHT_PROPS: [Prop; 5] = [
    Prop::Longhorn,
    Prop::PricklyPear,
    Prop::Fence,
    Prop::Longhorn,
    Prop::Fence,
];

impl Prop {
    fn width(self) -> u16 {
        match self {
            Prop::PricklyPear => art_width(PRICKLY_PEAR_ASCII),
            Prop::Windmill => art_width(WINDMILL_WHEEL_FRAMES[0]),
            Prop::StockTank => art_width(STOCK_TANK_ASCII),
            Prop::Longhorn => art_width(LONGHORN_ASCII),
            Prop::Fence => art_width(FENCE_ASCII),
        }
    }
}

/// A Texas Hill Country ranch: a stone house with a tin roof, a windmill,
/// prickly pear and longhorns in golden grass.
pub struct RanchScene {
    width: u16,
    height: u16,
    started: Instant,
}

impl RanchScene {
    const GROUND_HEIGHT: u16 = 7;
    const CHIMNEY_X_OFFSET: u16 = 10;

    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            started: Instant::now(),
        }
    }

    fn house_origin(&self, ground_y: u16) -> (u16, u16) {
        let house_x = (self.width / 2).saturating_sub(art_width(HOUSE_ASCII) / 2);
        let house_y = ground_y.saturating_sub(art_height(HOUSE_ASCII));
        (house_x, house_y)
    }

    /// Draws the grass and soil. `occupied` lists column ranges where something
    /// stands, so tall grass doesn't show through gaps in its art.
    fn render_ground(
        &self,
        renderer: &mut TerminalRenderer,
        ground_y: u16,
        occupied: &[Range<u16>],
        style: &RanchSceneStyle,
    ) -> io::Result<()> {
        // Sparse tufts of tall grass just above the ground line.
        for x in 0..self.width {
            let r = hash(x, 0xA11);
            if r < 18 && !occupied.iter().any(|range| range.contains(&x)) {
                let ch = if r < 9 { '"' } else { '\'' };
                renderer.render_char(
                    x,
                    ground_y.saturating_sub(1),
                    ch,
                    style.grass[x as usize % 3],
                )?;
            }
        }

        for y in 0..Self::GROUND_HEIGHT {
            for x in 0..self.width {
                let r = hash(x, y);
                let (ch, color) = if y == 0 {
                    let ch = match r {
                        0..=14 => '"',
                        15..=29 => ',',
                        30..=39 => '\'',
                        _ => '^',
                    };
                    (ch, style.grass[hash(x, 0xBEEF) as usize % 3])
                } else {
                    match r {
                        0..=11 => ('.', style.soil),
                        12..=17 => (':', style.soil),
                        18..=19 => ('o', style.rock),
                        _ => (' ', style.soil),
                    }
                };
                renderer.render_char(x, ground_y + y, ch, color)?;
            }
        }

        Ok(())
    }

    fn render_prop(
        &self,
        renderer: &mut TerminalRenderer,
        prop: Prop,
        x: u16,
        ground_y: u16,
        wind_speed_ms: f64,
        style: &RanchSceneStyle,
    ) -> io::Result<()> {
        match prop {
            Prop::PricklyPear => {
                let y = ground_y.saturating_sub(art_height(PRICKLY_PEAR_ASCII));
                render_art(renderer, PRICKLY_PEAR_ASCII, x, y, |_, ch| match ch {
                    'o' => style.cactus_fruit,
                    _ => style.cactus,
                })
            }
            Prop::Windmill => {
                let frame = match windmill_frame_ms(wind_speed_ms) {
                    Some(frame_ms) => {
                        (self.started.elapsed().as_millis() / frame_ms) as usize
                            % WINDMILL_WHEEL_FRAMES.len()
                    }
                    None => 0,
                };
                let wheel = WINDMILL_WHEEL_FRAMES[frame];
                let tower_y = ground_y.saturating_sub(art_height(WINDMILL_TOWER_ASCII));
                let wheel_y = tower_y.saturating_sub(art_height(wheel));
                render_art(renderer, WINDMILL_TOWER_ASCII, x, tower_y, |_, _| {
                    style.windmill
                })?;
                render_art(renderer, wheel, x, wheel_y, |_, ch| match ch {
                    'O' => style.trim,
                    _ => style.windmill,
                })
            }
            Prop::StockTank => {
                let y = ground_y.saturating_sub(art_height(STOCK_TANK_ASCII));
                render_art(renderer, STOCK_TANK_ASCII, x, y, |_, ch| match ch {
                    '~' => style.water,
                    _ => style.windmill,
                })
            }
            Prop::Longhorn => {
                let y = ground_y.saturating_sub(art_height(LONGHORN_ASCII));
                render_art(renderer, LONGHORN_ASCII, x, y, |row, ch| match (row, ch) {
                    (0, _) | (1, '`' | '\'' | '\\' | '/') => style.horn,
                    (_, '~' | '.') => style.hide_patch,
                    _ => style.hide,
                })
            }
            Prop::Fence => {
                let y = ground_y.saturating_sub(art_height(FENCE_ASCII));
                render_art(renderer, FENCE_ASCII, x, y, |_, ch| match ch {
                    '|' => style.fence_post,
                    _ => style.fence_wire,
                })
            }
        }
    }
}

impl Scene for RanchScene {
    fn id(&self) -> &'static str {
        "ranch"
    }

    fn update_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    fn layout(&self) -> SceneLayout {
        let ground_y = self.height.saturating_sub(Self::GROUND_HEIGHT);
        let (house_x, house_y) = self.house_origin(ground_y);

        SceneLayout {
            ground_y,
            chimney_pos: Some(ChimneyPosition {
                x: house_x + Self::CHIMNEY_X_OFFSET,
                y: house_y,
            }),
            width: self.width,
            height: self.height,
        }
    }

    fn render(&self, renderer: &mut TerminalRenderer, ctx: &SceneContext<'_>) -> io::Result<()> {
        let layout = self.layout();
        let ground_y = layout.ground_y;
        let (house_x, house_y) = self.house_origin(ground_y);
        let house_width = art_width(HOUSE_ASCII);
        let style = RanchSceneStyle::resolve(ctx);

        let props = place_outward(
            &LEFT_PROPS,
            &RIGHT_PROPS,
            Prop::width,
            house_x,
            house_width,
            self.width,
            PROP_GAP,
        );

        let occupied: Vec<Range<u16>> = std::iter::once(house_x..house_x + house_width)
            .chain(props.iter().map(|&(prop, x)| x..x + prop.width()))
            .collect();
        self.render_ground(renderer, ground_y, &occupied, &style)?;

        render_art(renderer, HOUSE_ASCII, house_x, house_y, |row, ch| {
            match (row, ch) {
                (0..=1, _) | (2, '|' | '#') => style.stone,
                (0..=4, _) => style.tin_roof,
                (_, '#') => style.window,
                (_, '[' | ']' | ':') => style.trim,
                (_, 'o') => style.brass,
                _ => style.wood,
            }
        })?;

        for (prop, x) in props {
            self.render_prop(renderer, prop, x, ground_y, ctx.wind_speed_ms, &style)?;
        }

        Ok(())
    }
}

/// How long each windmill wheel frame stays on screen at this wind speed,
/// or `None` when it's too calm for the wheel to turn.
fn windmill_frame_ms(wind_speed_ms: f64) -> Option<u128> {
    if wind_speed_ms < WINDMILL_CALM_MS {
        return None;
    }
    let (fastest, slowest) = WINDMILL_FRAME_MS_RANGE;
    Some((WINDMILL_SPEED_FACTOR / wind_speed_ms).clamp(fastest, slowest) as u128)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windmill_frames_match_in_size() {
        for frame in WINDMILL_WHEEL_FRAMES {
            assert_eq!(art_height(frame), art_height(WINDMILL_WHEEL_FRAMES[0]));
        }
    }

    #[test]
    fn windmill_speed_follows_wind() {
        assert_eq!(windmill_frame_ms(0.0), None);
        assert_eq!(windmill_frame_ms(0.4), None);
        assert_eq!(windmill_frame_ms(0.6), Some(1000));
        assert_eq!(windmill_frame_ms(5.0), Some(240));
        assert_eq!(windmill_frame_ms(40.0), Some(100));
    }

    #[test]
    fn chimney_sits_on_the_chimney() {
        let top_row = HOUSE_ASCII.lines().next().unwrap();
        let chimney = top_row.chars().nth(RanchScene::CHIMNEY_X_OFFSET as usize);
        assert_eq!(chimney, Some('_'));
    }
}
