use crate::scene::SceneContext;
use crossterm::style::Color;

#[derive(Clone, Copy)]
pub struct RanchSceneStyle {
    pub tin_roof: Color,
    pub stone: Color,
    pub wood: Color,
    pub window: Color,
    pub trim: Color,
    pub brass: Color,
    pub grass: [Color; 3],
    pub soil: Color,
    pub rock: Color,
    pub cactus: Color,
    pub cactus_fruit: Color,
    pub hide: Color,
    pub hide_patch: Color,
    pub horn: Color,
    pub windmill: Color,
    pub water: Color,
    pub fence_post: Color,
    pub fence_wire: Color,
}

impl RanchSceneStyle {
    pub fn resolve(ctx: &SceneContext<'_>) -> Self {
        let palette = ctx.palette;

        if ctx.conditions.sun.is_day {
            Self {
                tin_roof: Color::Grey,
                stone: palette.accent_secondary,
                wood: Color::Rgb {
                    r: 139,
                    g: 90,
                    b: 43,
                },
                window: Color::Cyan,
                trim: Color::White,
                brass: Color::Yellow,
                grass: [
                    Color::Rgb {
                        r: 218,
                        g: 165,
                        b: 32,
                    },
                    Color::Rgb {
                        r: 189,
                        g: 183,
                        b: 107,
                    },
                    Color::Rgb {
                        r: 240,
                        g: 220,
                        b: 130,
                    },
                ],
                soil: Color::Rgb {
                    r: 150,
                    g: 95,
                    b: 55,
                },
                rock: Color::Grey,
                cactus: Color::Green,
                cactus_fruit: Color::Magenta,
                hide: Color::Rgb {
                    r: 170,
                    g: 90,
                    b: 40,
                },
                hide_patch: Color::White,
                horn: Color::Rgb {
                    r: 245,
                    g: 235,
                    b: 210,
                },
                windmill: Color::Grey,
                water: Color::Blue,
                fence_post: Color::Rgb {
                    r: 139,
                    g: 90,
                    b: 43,
                },
                fence_wire: Color::DarkGrey,
            }
        } else {
            Self {
                tin_roof: Color::DarkGrey,
                stone: Color::Rgb {
                    r: 120,
                    g: 105,
                    b: 80,
                },
                wood: Color::Rgb {
                    r: 90,
                    g: 60,
                    b: 30,
                },
                window: Color::Yellow,
                trim: Color::Grey,
                brass: Color::DarkYellow,
                grass: [
                    Color::Rgb {
                        r: 120,
                        g: 95,
                        b: 30,
                    },
                    Color::Rgb {
                        r: 100,
                        g: 95,
                        b: 60,
                    },
                    Color::Rgb {
                        r: 130,
                        g: 115,
                        b: 70,
                    },
                ],
                soil: Color::Rgb {
                    r: 80,
                    g: 50,
                    b: 30,
                },
                rock: Color::DarkGrey,
                cactus: Color::DarkGreen,
                cactus_fruit: Color::DarkMagenta,
                hide: Color::Rgb {
                    r: 100,
                    g: 55,
                    b: 25,
                },
                hide_patch: Color::Grey,
                horn: Color::Rgb {
                    r: 160,
                    g: 150,
                    b: 130,
                },
                windmill: Color::DarkGrey,
                water: Color::DarkBlue,
                fence_post: Color::Rgb {
                    r: 90,
                    g: 60,
                    b: 30,
                },
                fence_wire: Color::DarkGrey,
            }
        }
    }
}
