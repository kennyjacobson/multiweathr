use crate::scene::SceneContext;
use crossterm::style::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

#[derive(Clone, Copy)]
pub struct SuburbSceneStyle {
    pub is_night: bool,
    pub saddleback: Color,
    pub saddleback_texture: Color,
    pub hills: Color,
    pub antenna: Color,
    pub beacon: Color,
    pub stucco: [Color; 4],
    pub roof_tile: [Color; 3],
    pub trim: Color,
    pub glass: Color,
    pub garage: Color,
    pub brass: Color,
    pub frond: Color,
    pub palm_trunk: Color,
    pub blossom: Color,
    pub bark: Color,
    pub lawn: Color,
    pub sidewalk: Color,
    pub asphalt: Color,
    pub lane: Color,
    pub car_paint: [Color; 5],
    pub car_glass: Color,
    pub tire: Color,
}

impl SuburbSceneStyle {
    pub fn resolve(ctx: &SceneContext<'_>) -> Self {
        if ctx.conditions.sun.is_day {
            Self {
                is_night: false,
                saddleback: rgb(150, 140, 105),
                saddleback_texture: rgb(110, 125, 80),
                hills: rgb(175, 160, 115),
                antenna: Color::Grey,
                beacon: Color::Red,
                stucco: [
                    rgb(245, 235, 215),
                    rgb(240, 220, 190),
                    rgb(230, 228, 218),
                    rgb(245, 222, 200),
                ],
                roof_tile: [rgb(205, 95, 55), rgb(185, 80, 50), rgb(215, 115, 75)],
                trim: rgb(120, 75, 45),
                glass: Color::Cyan,
                garage: Color::White,
                brass: Color::Yellow,
                frond: rgb(60, 165, 70),
                palm_trunk: rgb(165, 125, 80),
                blossom: rgb(160, 110, 220),
                bark: rgb(110, 75, 50),
                lawn: rgb(90, 190, 80),
                sidewalk: rgb(205, 205, 200),
                asphalt: Color::DarkGrey,
                lane: Color::White,
                car_paint: [
                    Color::White,
                    rgb(190, 195, 200),
                    rgb(200, 35, 40),
                    rgb(40, 80, 170),
                    rgb(215, 195, 155),
                ],
                car_glass: Color::Cyan,
                tire: Color::DarkGrey,
            }
        } else {
            Self {
                is_night: true,
                saddleback: rgb(80, 75, 65),
                saddleback_texture: rgb(60, 70, 50),
                hills: rgb(90, 85, 65),
                antenna: Color::DarkGrey,
                beacon: Color::Red,
                stucco: [
                    rgb(150, 145, 135),
                    rgb(145, 135, 120),
                    rgb(140, 140, 135),
                    rgb(150, 138, 125),
                ],
                roof_tile: [rgb(120, 60, 35), rgb(105, 50, 32), rgb(130, 70, 45)],
                trim: rgb(80, 50, 30),
                glass: Color::Yellow,
                garage: Color::Grey,
                brass: Color::DarkYellow,
                frond: rgb(25, 85, 35),
                palm_trunk: rgb(100, 75, 50),
                blossom: rgb(95, 65, 135),
                bark: rgb(65, 45, 30),
                lawn: rgb(35, 90, 35),
                sidewalk: Color::DarkGrey,
                asphalt: Color::DarkGrey,
                lane: Color::Grey,
                car_paint: [
                    Color::Grey,
                    rgb(120, 125, 130),
                    rgb(120, 25, 30),
                    rgb(30, 50, 105),
                    rgb(130, 118, 95),
                ],
                car_glass: Color::DarkGrey,
                tire: Color::DarkGrey,
            }
        }
    }
}
