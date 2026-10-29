//! The rows of the list: what one holds, how it is drawn, and how its lines are cut to fit.

use std::{ops::Range, sync::Arc};

use egui::{
    text::{LayoutJob, TextWrapping},
    vec2, Align2, Color32, CornerRadius, Stroke, StrokeKind,
};

use crate::PaletteStyle;

/// The gap between a row's edge and what is written in it.
const ROW_PADDING: f32 = 9.0;

/// The room round a choice's label, inside its ground when it is the one picked.
const CHOICE_PADDING: egui::Vec2 = vec2(5.0, 2.0);

/// The gap between two choices.
const CHOICE_GAP: f32 = 3.0;

/// The label of the choice picked, over its ink: the inks are light enough to be read as text
/// on the panel, so a dark label reads on any of them, on either theme.
const PICKED_CHOICE_TEXT: Color32 = Color32::from_rgb(0x14, 0x18, 0x20);

/// One line of the list: a title, a sentence under it, and the keyboard chord for the same
/// thing, if there is one.
#[derive(Clone, Debug, Default)]
pub struct Row {
    pub title: String,
    pub description: String,
    pub shortcut: String,
    /// The title's colour, for a row set apart from the others. `None` is the style's text.
    pub title_color: Option<Color32>,
    /// Written before the title in the muted colour - the number of a matching line, say.
    pub prefix: String,
    /// The byte ranges of the title written in the accent colour: where what was typed is.
    pub marks: Vec<Range<usize>>,
    /// Byte ranges of the title written in a colour of their own - the name of the choice
    /// picked, say. None of them overlaps a mark.
    pub inked: Vec<(Range<usize>, Color32)>,
    /// Ways to run the row, drawn as small labels against its right edge. ← and → move
    /// [`Row::picked`] among them while the row is highlighted, and a click on one runs the
    /// row with it - both reported by [`crate::CommandPalette::show_sections`]. The caller
    /// keeps which one is picked.
    pub choices: Vec<Choice>,
    /// The place in `choices` of the one Enter runs the row with, drawn on its ink.
    pub picked: usize,
    /// The title in the monospace font - a line of shell, say.
    pub title_mono: bool,
    /// A small dot of this colour before the title - whose a row is, or how it is doing.
    pub dot: Option<Color32>,
    /// Short notes against the right edge, left of the shortcut - what kind of thing the row
    /// is, how long ago it was run.
    pub asides: Vec<Aside>,
    /// Whether running the row leaves the palette open: a row whose result is shown beside
    /// the palette rather than somewhere it puts the palette away for.
    pub stays_open: bool,
}

/// A short note on a row's right side, in the detail size.
#[derive(Clone, Debug)]
pub struct Aside {
    pub text: String,
    /// `None` is the style's muted colour.
    pub ink: Option<Color32>,
    /// Drawn in a hairline box, as a tag rather than a remark.
    pub boxed: bool,
}

/// How wide a row's dot is, and the gap after it.
const DOT_DIAMETER: f32 = 7.0;
const DOT_GAP: f32 = 6.0;

/// The room round a boxed aside's text, inside its box.
const ASIDE_PADDING: egui::Vec2 = vec2(4.0, 0.0);

/// One of a row's ways to run it: its label, and the colour it is told apart by.
#[derive(Clone, Debug)]
pub struct Choice {
    pub label: String,
    pub ink: Color32,
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
            let ellipsis_width = line("…".to_string(), TextWrapping::no_max_width()).size().x;
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

/// A row's title on one line no wider than `width`: its prefix muted, its marks in the
/// accent colour, its inked ranges in theirs, the rest in the row's colour, and an ellipsis
/// where it was cut.
fn title_job(row: &Row, style: &PaletteStyle, width: f32) -> LayoutJob {
    let font = if row.title_mono {
        egui::FontId::monospace(style.title_size)
    } else {
        egui::FontId::proportional(style.title_size)
    };
    let format = |color: Color32| egui::TextFormat::simple(font.clone(), color);
    let ink = row.title_color.unwrap_or(style.text);
    let mut job = LayoutJob::default();
    if !row.prefix.is_empty() {
        job.append(&row.prefix, 0.0, format(style.muted));
        job.append(" ", 0.0, format(style.muted));
    }
    let mut colored: Vec<(Range<usize>, Color32)> = row
        .marks
        .iter()
        .map(|mark| (mark.clone(), style.accent))
        .chain(row.inked.iter().cloned())
        .collect();
    colored.sort_by_key(|(range, _)| range.start);
    let mut written = 0;
    for (range, color) in colored {
        assert!(
            range.start >= written,
            "a row's marks and inked ranges overlap in {:?}",
            row.title
        );
        job.append(&row.title[written..range.start], 0.0, format(ink));
        job.append(&row.title[range.clone()], 0.0, format(color));
        written = range.end;
    }
    job.append(&row.title[written..], 0.0, format(ink));
    job.wrap = TextWrapping::truncate_at_width(width);
    job
}

/// A row of the list. Its two lines are cut to the row: a matching line of a file is as long
/// as whoever wrote it liked, and the painter clips nothing, so a line left whole ran out of
/// the palette's right edge and across the window behind it. With the row's response, the
/// place of the choice clicked, if one was.
pub(crate) fn draw_row(
    ui: &mut egui::Ui,
    row: &Row,
    description_kept: Kept,
    highlighted: bool,
    style: &PaletteStyle,
) -> (egui::Response, Option<usize>) {
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, style.row_height), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let mut choice_clicked = None;

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
        for aside in row.asides.iter().rev() {
            let right = rect.min.x + ROW_PADDING + text_width;
            text_width -= draw_aside(ui, aside, right, rect.center().y, style) + ROW_PADDING;
        }
        if !row.choices.is_empty() {
            let right = rect.min.x + ROW_PADDING + text_width;
            let (width, clicked) = draw_choices(ui, row, &response, right, rect.center().y, style);
            text_width -= width + ROW_PADDING;
            choice_clicked = clicked;
        }
        let title_left = match row.dot {
            Some(_) => ROW_PADDING + DOT_DIAMETER + DOT_GAP,
            None => ROW_PADDING,
        };
        let title = ui.painter().layout_job(title_job(
            row,
            style,
            text_width - (title_left - ROW_PADDING),
        ));
        // A row with no second line has its title on the row's middle, as the chips and the
        // shortcut are.
        let title_top = if row.description.is_empty() {
            (rect.height() - title.size().y) / 2.0
        } else {
            style.title_top
        };
        if let Some(dot) = row.dot {
            let center = rect.min
                + vec2(
                    ROW_PADDING + DOT_DIAMETER / 2.0,
                    title_top + title.size().y / 2.0,
                );
            ui.painter().circle_filled(center, DOT_DIAMETER / 2.0, dot);
        }
        ui.painter()
            .galley(rect.min + vec2(title_left, title_top), title, style.text);
        if row.description.is_empty() {
            return (response, choice_clicked);
        }
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
    (response, choice_clicked)
}

/// An aside ending at `right`, centred on `center_y`. How wide it is.
fn draw_aside(
    ui: &egui::Ui,
    aside: &Aside,
    right: f32,
    center_y: f32,
    style: &PaletteStyle,
) -> f32 {
    let ink = aside.ink.unwrap_or(style.muted);
    let text = ui.painter().layout_no_wrap(
        aside.text.clone(),
        egui::FontId::proportional(style.detail_size),
        ink,
    );
    let padding = if aside.boxed {
        ASIDE_PADDING
    } else {
        egui::Vec2::ZERO
    };
    let size = text.size() + 2.0 * padding;
    let rect = egui::Rect::from_min_size(egui::pos2(right - size.x, center_y - size.y / 2.0), size);
    if aside.boxed {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, style.line),
            StrokeKind::Inside,
        );
    }
    ui.painter().galley(rect.min + padding, text, ink);
    size.x
}

/// A row's choices in a line ending at `right`, centred on `center_y`: the one picked on its
/// ink, the others in the muted colour - no outline, nor ground until hovered. How wide they
/// are, and which one was clicked.
fn draw_choices(
    ui: &mut egui::Ui,
    row: &Row,
    row_response: &egui::Response,
    right: f32,
    center_y: f32,
    style: &PaletteStyle,
) -> (f32, Option<usize>) {
    assert!(
        row.picked < row.choices.len(),
        "the row's picked choice {} is not one of its {} choices",
        row.picked,
        row.choices.len()
    );
    let font = egui::FontId::proportional(style.detail_size);
    let labels: Vec<_> = row
        .choices
        .iter()
        .map(|choice| {
            ui.painter()
                .layout_no_wrap(choice.label.clone(), font.clone(), Color32::PLACEHOLDER)
        })
        .collect();
    let width = labels
        .iter()
        .map(|label| label.size().x + 2.0 * CHOICE_PADDING.x)
        .sum::<f32>()
        + CHOICE_GAP * (labels.len() - 1) as f32;
    let mut x = right - width;
    let mut clicked = None;
    for (at, (choice, label)) in row.choices.iter().zip(labels).enumerate() {
        let size = label.size() + 2.0 * CHOICE_PADDING;
        let chip = egui::Rect::from_min_size(egui::pos2(x, center_y - size.y / 2.0), size);
        x += size.x + CHOICE_GAP;
        let response = ui
            .interact(
                chip,
                row_response.id.with(("choice", at)),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            clicked = Some(at);
        }
        let text = if at == row.picked {
            ui.painter()
                .rect_filled(chip, CornerRadius::same(3), choice.ink);
            PICKED_CHOICE_TEXT
        } else if response.hovered() {
            ui.painter()
                .rect_filled(chip, CornerRadius::same(3), style.selected);
            style.text
        } else {
            style.muted
        };
        ui.painter().galley(chip.min + CHOICE_PADDING, label, text);
    }
    (width, clicked)
}
