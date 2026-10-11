use crate::scene::SceneContext;
use crossterm::style::Color;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

#[derive(Clone, Copy)]
pub struct BeachSceneStyle {
    pub ocean: [Color; 3],
    pub foam: [Color; 2],
    pub wet_sand: Color,
    pub sand: Color,
    pub sand_texture: Color,
    pub pier_wood: Color,
    pub piling: Color,
    pub lamp: Color,
    pub cafe_wall: Color,
    pub cafe_roof: Color,
    pub window: Color,
    pub tower_body: Color,
    pub tower_roof: Color,
    pub tower_number: Color,
    pub tower_glass: Color,
    pub tower_legs: Color,
    pub umbrella: [Color; 3],
    pub pole: Color,
    pub net: Color,
    pub ball: Color,
    pub boards: [Color; 3],
    pub frond: Color,
    pub palm_trunk: Color,
    pub surfer: Color,
    pub surfboard: Color,
}

impl BeachSceneStyle {
    pub fn resolve(ctx: &SceneContext<'_>) -> Self {
        if ctx.conditions.sun.is_day {
            Self {
                ocean: [rgb(30, 100, 170), rgb(50, 135, 195), rgb(95, 165, 215)],
                foam: [Color::White, rgb(190, 235, 245)],
                wet_sand: rgb(190, 160, 110),
                sand: rgb(235, 215, 165),
                sand_texture: rgb(205, 180, 130),
                pier_wood: rgb(165, 125, 85),
                piling: rgb(120, 90, 60),
                lamp: Color::Grey,
                cafe_wall: Color::White,
                cafe_roof: rgb(205, 50, 50),
                window: Color::Cyan,
                tower_body: rgb(110, 185, 230),
                tower_roof: Color::White,
                tower_number: rgb(205, 50, 50),
                tower_glass: Color::Cyan,
                tower_legs: Color::Grey,
                umbrella: [rgb(220, 50, 50), Color::White, rgb(250, 200, 50)],
                pole: Color::White,
                net: Color::White,
                ball: Color::Yellow,
                boards: [rgb(250, 140, 60), rgb(80, 200, 200), rgb(240, 230, 120)],
                frond: rgb(60, 165, 70),
                palm_trunk: rgb(165, 125, 80),
                surfer: rgb(240, 190, 140),
                surfboard: Color::White,
            }
        } else {
            Self {
                ocean: [rgb(15, 45, 85), rgb(20, 60, 100), rgb(35, 75, 115)],
                foam: [Color::Grey, rgb(110, 130, 140)],
                wet_sand: rgb(95, 80, 60),
                sand: rgb(125, 112, 88),
                sand_texture: rgb(105, 92, 70),
                pier_wood: rgb(100, 75, 50),
                piling: rgb(70, 55, 40),
                lamp: Color::Yellow,
                cafe_wall: Color::Grey,
                cafe_roof: rgb(120, 35, 35),
                window: Color::Yellow,
                tower_body: rgb(55, 95, 125),
                tower_roof: Color::Grey,
                tower_number: rgb(120, 35, 35),
                tower_glass: rgb(45, 48, 55),
                tower_legs: Color::DarkGrey,
                umbrella: [rgb(120, 35, 35), Color::Grey, rgb(140, 115, 35)],
                pole: Color::Grey,
                net: Color::Grey,
                ball: Color::DarkYellow,
                boards: [rgb(140, 80, 35), rgb(45, 110, 110), rgb(135, 130, 70)],
                frond: rgb(25, 85, 35),
                palm_trunk: rgb(100, 75, 50),
                surfer: rgb(140, 110, 85),
                surfboard: Color::Grey,
            }
        }
    }
}
