//! Sections - the list cut into groups of rows, each under a header with its label and count,
//! and the placeholders a group keeps its room with while its rows are still coming in.

use egui::{text::LayoutJob, vec2, Align2, Color32, CornerRadius};

use crate::{rows::Row, Kept, PaletteStyle};

/// The gap between a section's edge and what is written in it, the same as a row's.
const HEADER_PADDING: f32 = 9.0;

/// How wide the spinner of a section still searching is.
const SPINNER_SIZE: f32 = 9.0;

/// One group of the list's rows, under its header.
pub struct Section<'a> {
    /// `None` lists the rows bare, as the list was before it had sections.
    pub header: Option<SectionHeader<'a>>,
    pub rows: &'a [Row],
    /// Which end of a row's second line is kept when it is too long for the row.
    pub description_kept: Kept,
    /// Rows still to come, drawn as dim bars under the rows so the list keeps its height
    /// while they arrive. The keyboard skips them.
    pub placeholders: usize,
}

/// What a section's header says about it.
pub struct SectionHeader<'a> {
    /// Written in capitals.
    pub label: &'a str,
    /// How many rows the section has, as the caller words it: `12`, `12…`, `48+`.
    pub count: &'a str,
    /// Whether the section is still looking for rows, which a spinner beside the count says.
    pub busy: bool,
    /// Said against the header's right edge - the rows the section has beyond those shown,
    /// and how to reach them. Clicking it is reported by [`crate::CommandPalette::show_sections`].
    pub more: Option<&'a str>,
}

/// How tall `sections` are when laid out one under the other: their headers, rows and
/// placeholders, and the layout's gap between each pair. Counting the items alone left the
/// area a gap short per row, which put a scrollbar on a list of three rows that all fit.
pub(crate) fn sections_height(sections: &[Section], style: &PaletteStyle, gap: f32) -> f32 {
    let headers = sections
        .iter()
        .filter(|section| section.header.is_some())
        .count();
    let rows: usize = sections
        .iter()
        .map(|section| section.rows.len() + section.placeholders)
        .sum();
    let items = headers + rows;
    headers as f32 * style.header_height
        + rows as f32 * style.row_height
        + items.saturating_sub(1) as f32 * gap
}

/// A section's header: its label in small capitals, its count and spinner after it, and what
/// it has beyond its rows against the right edge. Whether the right side was clicked.
pub(crate) fn draw_header(ui: &mut egui::Ui, header: &SectionHeader, style: &PaletteStyle) -> bool {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, style.header_height), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return false;
    }
    let small = egui::FontId::proportional(style.detail_size);
    // The label sits at the bottom of the header, the room above it parting it from the
    // section before.
    let baseline_y = rect.max.y - 4.0;

    let mut job = LayoutJob::default();
    job.append(
        &header.label.to_uppercase(),
        0.0,
        egui::TextFormat {
            font_id: small.clone(),
            color: style.muted,
            extra_letter_spacing: 0.9,
            ..Default::default()
        },
    );
    job.append(
        header.count,
        8.0,
        egui::TextFormat::simple(small.clone(), style.muted),
    );
    let label = ui.painter().layout_job(job);
    let label_at = egui::pos2(rect.min.x + HEADER_PADDING, baseline_y - label.size().y);
    let label_width = label.size().x;
    let label_height = label.size().y;
    ui.painter().galley(label_at, label, style.muted);

    if header.busy {
        let center = egui::pos2(
            label_at.x + label_width + 8.0 + SPINNER_SIZE / 2.0,
            label_at.y + label_height / 2.0,
        );
        egui::Spinner::new()
            .size(SPINNER_SIZE)
            .color(style.accent)
            .paint_at(
                ui,
                egui::Rect::from_center_size(center, vec2(SPINNER_SIZE, SPINNER_SIZE)),
            );
    }

    let Some(more) = header.more else {
        return false;
    };
    let more = ui
        .painter()
        .layout_no_wrap(more.to_string(), small, style.muted);
    let more_rect = Align2::RIGHT_BOTTOM.anchor_size(
        egui::pos2(rect.max.x - HEADER_PADDING, baseline_y),
        more.size(),
    );
    let response = ui
        .interact(
            more_rect,
            ui.id().with(("section-more", header.label)),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let ink = if response.hovered() {
        style.text
    } else {
        style.muted
    };
    ui.painter().galley(more_rect.min, more, ink);
    response.clicked()
}

/// A row still to come: a dim bar where its title would be.
pub(crate) fn draw_placeholder(ui: &mut egui::Ui, style: &PaletteStyle) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, style.row_height), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let bar = rect.shrink2(vec2(HEADER_PADDING, style.row_height * 0.25));
    ui.painter()
        .rect_filled(bar, CornerRadius::same(4), placeholder_color(style));
}

/// The placeholders' ground: the highlight's, faded, so a bar reads as room rather than a row.
fn placeholder_color(style: &PaletteStyle) -> Color32 {
    style.selected.gamma_multiply(0.6)
}
