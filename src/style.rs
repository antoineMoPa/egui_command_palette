//! The palette's colours and the sizes of its rows.

use egui::Color32;

#[derive(Clone, Debug)]
pub struct PaletteStyle {
    pub panel: Color32,
    pub line: Color32,
    pub selected: Color32,
    pub accent: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub shadow: Color32,
    pub row_height: f32,
    pub title_size: f32,
    pub detail_size: f32,
    /// How far under the top of a row its second line starts.
    pub detail_top: f32,
}

impl Default for PaletteStyle {
    fn default() -> Self {
        Self {
            panel: Color32::from_rgb(31, 32, 35),
            line: Color32::from_gray(67),
            selected: Color32::from_rgb(48, 51, 57),
            accent: Color32::from_rgb(104, 151, 255),
            text: Color32::WHITE,
            muted: Color32::GRAY,
            shadow: Color32::from_black_alpha(70),
            row_height: 42.0,
            title_size: 14.0,
            detail_size: 11.0,
            detail_top: 23.0,
        }
    }
}
