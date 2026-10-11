use crate::scene::SceneContext;
use crossterm::style::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

#[derive(Clone, Copy)]
pub struct LakeHouseSceneStyle {
    pub mountain: Color,
    pub mountain_texture: Color,
    pub snow: Color,
    pub roof: Color,
    pub wood: Color,
    pub glass: Color,
    pub trim: Color,
    pub stovepipe: Color,
    pub pine: Color,
    pub trunk: Color,
    pub sign_board: Color,
    pub sign_text: Color,
    pub shore_grass: Color,
    pub sand: Color,
    pub water: [Color; 3],
    pub sail: Color,
    pub hull: Color,
}

impl LakeHouseSceneStyle {
    pub fn resolve(ctx: &SceneContext<'_>) -> Self {
        if ctx.conditions.sun.is_day {
            Self {
                mountain: rgb(150, 170, 200),
                mountain_texture: rgb(120, 140, 170),
                snow: Color::White,
                roof: rgb(185, 100, 55),
                wood: rgb(181, 136, 99),
                glass: Color::Cyan,
                trim: Color::White,
                stovepipe: Color::DarkGrey,
                pine: rgb(45, 150, 75),
                trunk: rgb(110, 70, 40),
                sign_board: Color::White,
                sign_text: rgb(200, 30, 80),
                shore_grass: Color::Green,
                sand: rgb(230, 205, 150),
                // Bear Lake's famous turquoise.
                water: [rgb(64, 224, 208), rgb(0, 170, 180), rgb(150, 235, 225)],
                sail: Color::White,
                hull: rgb(180, 50, 50),
            }
        } else {
            Self {
                mountain: rgb(80, 92, 125),
                mountain_texture: rgb(60, 70, 100),
                snow: Color::Grey,
                roof: rgb(120, 70, 40),
                wood: rgb(100, 75, 55),
                glass: Color::Yellow,
                trim: Color::Grey,
                stovepipe: Color::DarkGrey,
                pine: rgb(25, 85, 45),
                trunk: rgb(60, 40, 25),
                sign_board: Color::Grey,
                sign_text: rgb(120, 20, 50),
                shore_grass: Color::DarkGreen,
                sand: rgb(110, 95, 70),
                water: [rgb(20, 80, 90), rgb(10, 60, 75), rgb(40, 100, 110)],
                sail: Color::Grey,
                hull: rgb(100, 30, 30),
            }
        }
    }
}
