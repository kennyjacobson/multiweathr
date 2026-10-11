use crate::scene::SceneContext;
use crossterm::style::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

#[derive(Clone, Copy)]
pub struct CitySceneStyle {
    pub is_night: bool,
    pub mountain: Color,
    pub mountain_texture: Color,
    pub snow: Color,
    pub stone: Color,
    pub capitol_window: Color,
    pub towers: [Color; 4],
    pub tower_glass: Color,
    pub window_lit: Color,
    pub window_dark: Color,
    pub antenna: Color,
    pub beacon: Color,
    pub flag_blue: Color,
    pub flag_red: Color,
    pub flag_white: Color,
    pub sidewalk: Color,
    pub asphalt: Color,
    pub lane: Color,
    pub rail: Color,
    pub grass: Color,
    pub train_body: Color,
    pub train_stripe: Color,
    pub train_window: Color,
    pub wheel: Color,
}

impl CitySceneStyle {
    pub fn resolve(ctx: &SceneContext<'_>) -> Self {
        if ctx.conditions.sun.is_day {
            Self {
                is_night: false,
                mountain: rgb(150, 170, 200),
                mountain_texture: rgb(120, 140, 170),
                snow: Color::White,
                stone: rgb(230, 228, 215),
                capitol_window: Color::Cyan,
                towers: [
                    rgb(170, 175, 185),
                    rgb(150, 160, 175),
                    rgb(190, 180, 165),
                    rgb(125, 135, 150),
                ],
                tower_glass: rgb(140, 185, 215),
                window_lit: rgb(250, 210, 90),
                window_dark: rgb(45, 48, 55),
                antenna: Color::Grey,
                beacon: Color::Red,
                flag_blue: rgb(40, 70, 170),
                flag_red: rgb(205, 35, 45),
                flag_white: Color::White,
                sidewalk: Color::Grey,
                asphalt: Color::DarkGrey,
                lane: Color::Yellow,
                rail: Color::Grey,
                grass: Color::Green,
                train_body: Color::White,
                train_stripe: rgb(205, 35, 45),
                train_window: Color::Cyan,
                wheel: Color::DarkGrey,
            }
        } else {
            Self {
                is_night: true,
                mountain: rgb(80, 92, 125),
                mountain_texture: rgb(60, 70, 100),
                snow: Color::Grey,
                stone: rgb(150, 150, 145),
                capitol_window: Color::Yellow,
                towers: [
                    rgb(75, 78, 88),
                    rgb(65, 70, 82),
                    rgb(85, 80, 75),
                    rgb(55, 60, 70),
                ],
                tower_glass: rgb(45, 48, 55),
                window_lit: rgb(250, 210, 90),
                window_dark: rgb(45, 48, 55),
                antenna: Color::DarkGrey,
                beacon: Color::Red,
                flag_blue: rgb(30, 45, 110),
                flag_red: rgb(130, 25, 30),
                flag_white: Color::Grey,
                sidewalk: Color::DarkGrey,
                asphalt: Color::DarkGrey,
                lane: Color::DarkYellow,
                rail: Color::DarkGrey,
                grass: Color::DarkGreen,
                train_body: Color::Grey,
                train_stripe: rgb(130, 25, 30),
                train_window: Color::Yellow,
                wheel: Color::DarkGrey,
            }
        }
    }
}
