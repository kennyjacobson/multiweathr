mod style;

use crate::render::TerminalRenderer;
use crate::scene::art::{
    MountainColors, art_height, art_width, hash, place_outward, render_art_clipped,
    render_art_opaque, render_mountains,
};
use crate::scene::{Scene, SceneContext, SceneLayout};
use crossterm::style::Color;
use std::io;
use std::time::Instant;
use style::SuburbSceneStyle;

const HOUSE_ASCII: &str = include_str!("assets/house.txt");
const NEIGHBOR_ASCII: &str = include_str!("assets/neighbor.txt");
const JACARANDA_ASCII: &str = include_str!("assets/jacaranda.txt");
const PALM_TRUNK_ASCII: &str = include_str!("assets/palm_trunk.txt");
/// Palm crowns: still, a light breeze, and a strong wind.
const PALM_CROWNS: [&str; 3] = [
    include_str!("assets/palm_calm.txt"),
    include_str!("assets/palm_wind_1.txt"),
    include_str!("assets/palm_wind_2.txt"),
];
const SADDLEBACK_ASCII: &str = include_str!("assets/saddleback.txt");
const CAR_ASCII: &str = include_str!("assets/car.txt");

/// Tallest the foothills get, in rows above the street.
const HILLS_MAX_HEIGHT: u16 = 5;
/// Saddleback sits this many columns right of center, behind the house.
const SADDLEBACK_X_OFFSET: i32 = 6;
/// How far Saddleback's base is raised above the street, at most.
const SADDLEBACK_MAX_LIFT: u16 = 4;
/// Below this wind speed (m/s, about 2 mph) the palms stand still.
const PALM_CALM_MS: f64 = 1.0;
/// From this wind speed (m/s, about 11 mph) the palms thrash in the wind.
const PALM_STRONG_MS: f64 = 5.0;
/// Sway frame time is this divided by the wind speed in m/s.
const PALM_SPEED_FACTOR: f64 = 2000.0;
/// Fastest and slowest sway frame times, in milliseconds.
const PALM_FRAME_MS_RANGE: (f64, f64) = (150.0, 1200.0);
/// A car drives by once per this many seconds...
const CAR_PERIOD_SECS: f64 = 25.0;
/// ...moving this many columns per second.
const CAR_SPEED: f64 = 22.0;
/// Empty columns between props.
const PROP_GAP: u16 = 2;

/// Something beside the house: neighbors and trees.
#[derive(Clone, Copy)]
enum Prop {
    Palm,
    Jacaranda,
    Neighbor,
}

// Props are placed outward from the house, cycling until the screen is full.
const LEFT_PROPS: [Prop; 6] = [
    Prop::Palm,
    Prop::Neighbor,
    Prop::Jacaranda,
    Prop::Neighbor,
    Prop::Palm,
    Prop::Neighbor,
];
const RIGHT_PROPS: [Prop; 6] = [
    Prop::Jacaranda,
    Prop::Neighbor,
    Prop::Palm,
    Prop::Neighbor,
    Prop::Palm,
    Prop::Neighbor,
];

impl Prop {
    fn width(self) -> u16 {
        match self {
            Prop::Palm => PALM_CROWNS
                .iter()
                .map(|crown| art_width(crown))
                .max()
                .unwrap_or(0),
            Prop::Jacaranda => art_width(JACARANDA_ASCII),
            Prop::Neighbor => art_width(NEIGHBOR_ASCII),
        }
    }
}

/// The colors of one house, so neighbors can differ.
#[derive(Clone, Copy)]
struct HousePaint {
    stucco: Color,
    roof: Color,
    /// Whether its windows glow at night.
    lit: bool,
}

/// A South Orange County street in Rancho Santa Margarita: Spanish-style
/// houses with tile roofs, palms that sway in the wind, jacarandas, and
/// Saddleback Mountain behind.
pub struct SuburbScene {
    width: u16,
    height: u16,
    started: Instant,
}

impl SuburbScene {
    /// Street rows: lawn, sidewalk, four rows of road, and the far curb.
    const GROUND_HEIGHT: u16 = 7;

    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            started: Instant::now(),
        }
    }

    fn render_saddleback(
        &self,
        renderer: &mut TerminalRenderer,
        ground_y: u16,
        style: &SuburbSceneStyle,
    ) -> io::Result<()> {
        let art_h = art_height(SADDLEBACK_ASCII);
        // Raise the mountain so it peeks over the rooftops, if there's sky to spare.
        let lift = ground_y.saturating_sub(art_h + 3).min(SADDLEBACK_MAX_LIFT);
        let Some(top) = ground_y.checked_sub(art_h + lift) else {
            return Ok(());
        };
        let left =
            self.width as i32 / 2 - art_width(SADDLEBACK_ASCII) as i32 / 2 + SADDLEBACK_X_OFFSET;
        let beacon_on = style.is_night && (self.started.elapsed().as_millis() / 800) % 2 == 0;

        let rows: Vec<&str> = SADDLEBACK_ASCII.lines().collect();
        let base = rows.last().copied().unwrap_or("");
        // Below the art, keep filling the base down to the street.
        for (i, line) in rows
            .iter()
            .chain(std::iter::repeat_n(&base, lift as usize))
            .enumerate()
        {
            let y = top + i as u16;
            let chars: Vec<char> = line.chars().collect();
            let Some(first) = chars.iter().position(|&ch| ch != ' ') else {
                continue;
            };
            let last = chars.iter().rposition(|&ch| ch != ' ').unwrap_or(first);
            let below_art = i >= rows.len();
            for (j, &ch) in chars.iter().enumerate().take(last + 1).skip(first) {
                let col = left + j as i32;
                if !(0..self.width as i32).contains(&col) {
                    continue;
                }
                let (ch, color) = match ch {
                    _ if below_art => (' ', style.saddleback),
                    'i' if beacon_on => ('i', style.beacon),
                    'i' => ('i', style.antenna),
                    ' ' if hash(col as u16, y) < 12 => ('.', style.saddleback_texture),
                    _ => (ch, style.saddleback),
                };
                renderer.render_char(col as u16, y, ch, color)?;
            }
        }

        Ok(())
    }

    fn render_house(
        &self,
        renderer: &mut TerminalRenderer,
        ascii: &str,
        x: u16,
        ground_y: u16,
        paint: HousePaint,
        style: &SuburbSceneStyle,
    ) -> io::Result<()> {
        let y = ground_y.saturating_sub(art_height(ascii));
        // The roof is every row down to the eave line, the first that starts with '/_'.
        let roof_rows = ascii
            .lines()
            .position(|line| line.trim_start().starts_with("/_"))
            .unwrap_or(0);
        let glass = if style.is_night && !paint.lit {
            style.trim
        } else {
            style.glass
        };
        render_art_opaque(renderer, ascii, x, y, |row, ch| match ch {
            _ if row <= roof_rows => paint.roof,
            '#' => glass,
            '=' => style.garage,
            'o' => style.brass,
            '.' | '-' => style.trim,
            _ => paint.stucco,
        })
    }

    fn render_palm(
        &self,
        renderer: &mut TerminalRenderer,
        x: u16,
        ground_y: u16,
        wind_speed_ms: f64,
        style: &SuburbSceneStyle,
    ) -> io::Result<()> {
        let trunk_y = ground_y.saturating_sub(art_height(PALM_TRUNK_ASCII));
        render_art_opaque(renderer, PALM_TRUNK_ASCII, x, trunk_y, |_, _| {
            style.palm_trunk
        })?;

        let crown = PALM_CROWNS[palm_crown(wind_speed_ms, self.started.elapsed().as_millis())];
        let crown_y = trunk_y.saturating_sub(art_height(crown));
        render_art_clipped(renderer, crown, x as i32, crown_y, |_, ch| match ch {
            '|' => style.palm_trunk,
            _ => style.frond,
        })
    }

    fn render_street(
        &self,
        renderer: &mut TerminalRenderer,
        ground_y: u16,
        style: &SuburbSceneStyle,
    ) -> io::Result<()> {
        for x in 0..self.width {
            let road = |row: u16| {
                if hash(x, row) < 5 {
                    ('.', style.asphalt)
                } else {
                    (' ', style.asphalt)
                }
            };
            let rows = [
                (
                    match hash(x, 0) {
                        0..=24 => '"',
                        25..=44 => ',',
                        _ => '^',
                    },
                    style.lawn,
                ),
                (if x % 5 == 0 { '|' } else { '_' }, style.sidewalk),
                road(2),
                road(3),
                if x % 8 < 4 {
                    ('-', style.lane)
                } else {
                    road(4)
                },
                road(5),
                ('_', style.sidewalk),
            ];
            for (row, (ch, color)) in rows.into_iter().enumerate() {
                renderer.render_char(x, ground_y + row as u16, ch, color)?;
            }
        }

        Ok(())
    }

    fn render_car(
        &self,
        renderer: &mut TerminalRenderer,
        ground_y: u16,
        style: &SuburbSceneStyle,
    ) -> io::Result<()> {
        let elapsed = self.started.elapsed().as_secs_f64();
        let x = ((elapsed % CAR_PERIOD_SECS) * CAR_SPEED) as i32 - art_width(CAR_ASCII) as i32;
        if x >= self.width as i32 {
            return Ok(());
        }

        // A different car each time one drives by.
        let pass = (elapsed / CAR_PERIOD_SECS) as u16;
        let paint = style.car_paint[hash(pass, 13) as usize % style.car_paint.len()];
        // Wheels roll along the fourth road row.
        let y = ground_y + 5 - (art_height(CAR_ASCII) - 1);
        render_art_clipped(renderer, CAR_ASCII, x, y, |_, ch| match ch {
            '#' => style.car_glass,
            '(' | ')' => style.tire,
            _ => paint,
        })
    }
}

impl Scene for SuburbScene {
    fn id(&self) -> &'static str {
        "suburb"
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
        let style = SuburbSceneStyle::resolve(ctx);
        let house_width = art_width(HOUSE_ASCII);
        let house_x = (self.width / 2).saturating_sub(house_width / 2);

        render_mountains(
            renderer,
            self.width,
            ground_y,
            HILLS_MAX_HEIGHT.min(ground_y / 3),
            &MountainColors {
                ridge: style.hills,
                texture: style.saddleback_texture,
                snow: style.hills,
            },
        )?;
        self.render_saddleback(renderer, ground_y, &style)?;

        self.render_house(
            renderer,
            HOUSE_ASCII,
            house_x,
            ground_y,
            HousePaint {
                stucco: style.stucco[0],
                roof: style.roof_tile[0],
                lit: true,
            },
            &style,
        )?;

        let props = place_outward(
            &LEFT_PROPS,
            &RIGHT_PROPS,
            Prop::width,
            house_x,
            house_width,
            self.width,
            PROP_GAP,
        );
        for (prop, x) in props {
            match prop {
                Prop::Palm => self.render_palm(renderer, x, ground_y, ctx.wind_speed_ms, &style)?,
                Prop::Jacaranda => {
                    let y = ground_y.saturating_sub(art_height(JACARANDA_ASCII));
                    render_art_opaque(renderer, JACARANDA_ASCII, x, y, |_, ch| match ch {
                        '@' | '.' | '\'' => style.blossom,
                        _ => style.bark,
                    })?
                }
                Prop::Neighbor => {
                    // Each neighbor gets its own stucco, roof and porch light.
                    let paint = HousePaint {
                        stucco: style.stucco[hash(x, 21) as usize % style.stucco.len()],
                        roof: style.roof_tile[hash(x, 22) as usize % style.roof_tile.len()],
                        lit: hash(x, 23) < 70,
                    };
                    self.render_house(renderer, NEIGHBOR_ASCII, x, ground_y, paint, &style)?
                }
            }
        }

        self.render_street(renderer, ground_y, &style)?;
        self.render_car(renderer, ground_y, &style)?;

        Ok(())
    }
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

    #[test]
    fn house_roof_ends_at_the_eaves() {
        let eaves = HOUSE_ASCII
            .lines()
            .position(|line| line.trim_start().starts_with("/_"));
        assert_eq!(eaves, Some(3));
    }
}
