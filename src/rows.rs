//! The rows of the list: what one holds, how it is drawn, and how its lines are cut to fit.

use std::sync::Arc;

use egui::{
    text::{LayoutJob, TextWrapping},
    vec2, Align2, Color32, CornerRadius, Stroke, StrokeKind,
};

use crate::PaletteStyle;

/// The gap between a row's edge and what is written in it.
const ROW_PADDING: f32 = 9.0;

/// One line of the list: a title, a sentence under it, and the keyboard chord for the same
/// thing, if there is one.
#[derive(Clone, Debug)]
pub struct Row {
    pub title: String,
    pub description: String,
    pub shortcut: String,
}

/// Which end of a text is kept when it is wider than the room it has.
#[derive(Clone, Copy, Debug)]
pub enum Kept {
    Start,
    End,
}

/// The ground and outline of the row the keyboard is on.
pub fn paint_highlight(ui: &egui::Ui, rect: egui::Rect, style: &PaletteStyle) {
    ui.painter()
        .rect_filled(rect, CornerRadius::same(5), style.selected);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(1.0, style.accent),
        StrokeKind::Inside,
    );
}

/// How tall `rows` rows are when laid out one under the other: the rows themselves, and the
/// layout's gap between each pair. Counting the rows alone left the area a gap short per row,
/// which put a scrollbar on a list of three rows that all fit.
pub(crate) fn rows_height(rows: usize, row_height: f32, gap: f32) -> f32 {
    rows as f32 * row_height + rows.saturating_sub(1) as f32 * gap
}

/// `text` on one line no wider than `width`, with an ellipsis where the rest of it was.
pub fn cut_to_width(
    painter: &egui::Painter,
    text: &str,
    font: egui::FontId,
    color: Color32,
    width: f32,
    kept: Kept,
) -> Arc<egui::Galley> {
    let line = |text: String, wrap: TextWrapping| {
        let mut job = LayoutJob::simple_singleline(text, font.clone(), color);
        job.wrap = wrap;
        painter.layout_job(job)
    };
    match kept {
        Kept::Start => line(text.to_string(), TextWrapping::truncate_at_width(width)),
        Kept::End => {
            let whole = line(text.to_string(), TextWrapping::no_max_width());
            if whole.size().x <= width {
                return whole;
            }
            let ellipsis_width = line("…".to_string(), TextWrapping::no_max_width())
                .size()
                .x;
            // The first glyph that the rest of the line fits from, beside the ellipsis.
            let glyphs = &whole.rows[0].glyphs;
            let kept_from = glyphs
                .iter()
                .position(|glyph| whole.size().x - glyph.pos.x <= width - ellipsis_width)
                .unwrap_or(glyphs.len());
            let end: String = glyphs[kept_from..].iter().map(|glyph| glyph.chr).collect();
            line(format!("…{end}"), TextWrapping::no_max_width())
        }
    }
}

/// A row of the list. Its two lines are cut to the row: a matching line of a file is as long
/// as whoever wrote it liked, and the painter clips nothing, so a line left whole ran out of
/// the palette's right edge and across the window behind it.
pub(crate) fn draw_row(
    ui: &mut egui::Ui,
    row: &Row,
    description_kept: Kept,
    highlighted: bool,
    style: &PaletteStyle,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, style.row_height), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    if ui.is_rect_visible(rect) {
        if highlighted {
            paint_highlight(ui, rect, style);
        }
        let small = egui::FontId::proportional(style.detail_size);
        let mut text_width = rect.width() - 2.0 * ROW_PADDING;
        // The keyboard's own way to the same command, against the right edge - and the two
        // lines stop short of it.
        if !row.shortcut.is_empty() {
            let shortcut = ui.painter().text(
                egui::pos2(rect.max.x - ROW_PADDING, rect.center().y),
                Align2::RIGHT_CENTER,
                &row.shortcut,
                small.clone(),
                style.muted,
            );
            text_width -= shortcut.width() + ROW_PADDING;
        }
        let title = cut_to_width(
            ui.painter(),
            &row.title,
            egui::FontId::proportional(style.title_size),
            style.text,
            text_width,
            Kept::Start,
        );
        ui.painter()
            .galley(rect.min + vec2(ROW_PADDING, 5.0), title, style.text);
        let description = cut_to_width(
            ui.painter(),
            &row.description,
            small,
            style.muted,
            text_width,
            description_kept,
        );
        ui.painter().galley(
            rect.min + vec2(ROW_PADDING, style.detail_top),
            description,
            style.muted,
        );
    }
    response
}
