//! History box - the earlier runs, drawn above the palette in a box of their own: newest at the
//! bottom next to the query line, older going up. At rest the box is faint, fading out towards
//! its top; in use it comes up brighter, and the list rolls down as the keyboard goes back so
//! the row it is on is always the bottom one, drawn fully opaque.

use egui::{
    emath::easing,
    epaint::{Mesh, RectShape, Shadow, TessellationOptions, Tessellator},
    CornerRadius, Stroke, StrokeKind,
};

use crate::{
    rows::draw_row,
    sections::{draw_header, SectionHeader},
    CommandPalette, Kept, PaletteStyle, Row,
};

/// How opaque the box is at its bottom, next to the query line, and at its top.
#[derive(Clone, Copy, Debug)]
struct Fade {
    bottom: f32,
    top: f32,
}

impl Fade {
    fn lerp(self, to: Self, t: f32) -> Self {
        Self {
            bottom: egui::lerp(self.bottom..=to.bottom, t),
            top: egui::lerp(self.top..=to.top, t),
        }
    }

    /// The opacity at height `y` of `rect`.
    fn at(self, rect: egui::Rect, y: f32) -> f32 {
        let up = ((rect.max.y - y) / rect.height()).clamp(0.0, 1.0);
        egui::lerp(self.bottom..=self.top, up)
    }
}

/// The fade at rest: a hint of the newest few, the rest gone.
const FADE_AT_REST: Fade = Fade {
    bottom: 0.3,
    top: 0.0,
};

/// The fade in use.
const FADE_IN_USE: Fade = Fade {
    bottom: 0.8,
    top: 0.3,
};

/// How long the fade takes to go from one to the other, either way, in seconds.
const FADE_TIME: f32 = 0.09;

/// The gap between the box and the palette under it.
const GAP: f32 = 8.0;

/// The room kept between the box and the top of the window.
const WINDOW_MARGIN: f32 = 12.0;

/// How many rows the box shows before its list scrolls.
const ROWS_SHOWN: usize = 8;

/// The room between the box's edge and its rows.
const MARGIN: f32 = 9.0;

/// The earlier runs, as the box draws them.
pub struct HistoryBox<'a> {
    /// Newest first.
    pub rows: &'a [Row],
    /// Written small at the bottom of the box.
    pub label: &'a str,
    /// Said in place of the rows when there are none.
    pub empty: &'a str,
    /// Whether the box is in use: brighter, with the row the keyboard is on shown.
    pub active: bool,
    /// The row the keyboard is on while the box is in use, by its place in `rows`. The rows
    /// newer than it are rolled off the bottom of the box.
    pub highlighted: usize,
    /// Which end of a row's second line is kept when it is too long for the row.
    pub description_kept: Kept,
}

impl CommandPalette {
    /// Draw `history` above the palette, which has to have been drawn this frame and sits low
    /// enough to leave it the room. The place in `history.rows`
    /// of the row clicked, while the box is in use.
    pub fn show_history(&mut self, ctx: &egui::Context, history: &HistoryBox) -> Option<usize> {
        let palette = self.rect?;
        let style = self.style.clone();
        let gap = ctx.global_style().spacing.item_spacing.y;
        let row_count = history.rows.len().max(1);
        let list_height =
            |rows: usize| rows as f32 * style.row_height + rows.saturating_sub(1) as f32 * gap;
        let wanted =
            list_height(row_count.min(ROWS_SHOWN)) + gap + style.header_height + 2.0 * MARGIN;
        let bottom = palette.min.y - GAP;
        let height = wanted.min(bottom - WINDOW_MARGIN).max(0.0);
        let rect = egui::Rect::from_min_max(
            egui::pos2(palette.min.x, bottom - height),
            egui::pos2(palette.max.x, bottom),
        );
        let t = ctx.animate_bool_with_time_and_easing(
            egui::Id::new("command-palette-history-fade"),
            history.active,
            FADE_TIME,
            easing::cubic_out,
        );
        let fade = FADE_AT_REST.lerp(FADE_IN_USE, t);
        // The row kept in view: the one the keyboard is on, or the newest at rest. Scrolled to
        // when that changes, and otherwise left where the pointer put it.
        let in_view = if history.active {
            history.highlighted
        } else {
            0
        };
        let scroll_to = self.history_in_view != Some(in_view);
        self.history_in_view = Some(in_view);

        let mut clicked = None;
        egui::Area::new("command-palette-history".into())
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                paint_faded_frame(ui, rect, &style, fade);
                let inner = rect.shrink(MARGIN);
                let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(inner));
                ui.set_clip_rect(inner);
                let list = egui::Rect::from_min_max(
                    inner.min,
                    egui::pos2(inner.max.x, inner.max.y - style.header_height - gap),
                );
                egui::ScrollArea::vertical()
                    .id_salt("command-palette-history-rows")
                    .max_height(list.height())
                    .min_scrolled_height(list.height())
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(&mut ui, |ui| {
                        // In use, the rows newer than the one the keyboard is on are rolled off
                        // the bottom, so that one sits next to the query line.
                        let first_shown = if history.active {
                            history.highlighted
                        } else {
                            0
                        };
                        // Short of the room, the rows sit at its bottom, next to the query line.
                        let rows_height = list_height(row_count - first_shown.min(row_count - 1));
                        ui.add_space((list.height() - rows_height).max(0.0));
                        if history.rows.is_empty() {
                            faded(ui, rect, fade, style.row_height, |ui| {
                                ui.label(egui::RichText::new(history.empty).color(style.muted));
                            });
                        }
                        // Oldest at the top.
                        for (index, row) in history.rows.iter().enumerate().rev() {
                            if index < first_shown {
                                continue;
                            }
                            let highlighted = history.active && index == history.highlighted;
                            // The row the keyboard is on is read, not hinted at.
                            let row_fade = if highlighted {
                                Fade {
                                    bottom: 1.0,
                                    top: 1.0,
                                }
                            } else {
                                fade
                            };
                            let response = faded(ui, rect, row_fade, style.row_height, |ui| {
                                draw_row(ui, row, history.description_kept, highlighted, &style).0
                            });
                            if index == in_view && scroll_to {
                                response.scroll_to_me(None);
                            }
                            if history.active && response.clicked() {
                                clicked = Some(index);
                            }
                        }
                    });
                ui.add_space(gap);
                faded(&mut ui, rect, fade, style.header_height, |ui| {
                    let header = SectionHeader {
                        label: history.label,
                        count: "",
                        busy: false,
                        more: None,
                    };
                    draw_header(ui, &header, &style);
                });
            });
        self.note_beside(rect);
        if t > 0.0 && t < 1.0 {
            ctx.request_repaint();
        }
        clicked
    }
}

/// Draw what `add` adds at the opacity the fade has where it goes: the middle of the next
/// `height` of `ui`.
fn faded<R>(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    fade: Fade,
    height: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let middle = ui.cursor().min.y + height / 2.0;
    ui.scope(|ui| {
        ui.multiply_opacity(fade.at(rect, middle));
        add(ui)
    })
    .inner
}

/// The box's ground, hairline and shadow, the palette's own, each point of them as opaque as
/// the fade is at its height. Tessellated here rather than painted as shapes, which take one
/// colour each: the fade runs down the box, not shape by shape.
fn paint_faded_frame(ui: &egui::Ui, rect: egui::Rect, style: &PaletteStyle, fade: Fade) {
    let corners = CornerRadius::same(8);
    let shadow = Shadow {
        offset: [0, -8],
        blur: 28,
        spread: 0,
        color: style.shadow,
    }
    .as_shape(rect, corners);
    let ground = RectShape::new(
        rect,
        corners,
        style.panel,
        Stroke::new(1.0, style.line),
        StrokeKind::Inside,
    );
    let mut tessellator = Tessellator::new(
        ui.ctx().pixels_per_point(),
        TessellationOptions::default(),
        // Nothing here is text, whose glyphs are what the texture's size is for.
        [1, 1],
        Vec::new(),
    );
    let mut mesh = Mesh::default();
    tessellator.tessellate_rect(&shadow, &mut mesh);
    tessellator.tessellate_rect(&ground, &mut mesh);
    for vertex in &mut mesh.vertices {
        vertex.color = vertex.color.gamma_multiply(fade.at(rect, vertex.pos.y));
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use egui::vec2;

    use super::*;

    #[test]
    fn the_fade_runs_from_the_bottom_to_the_top() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 100.0), vec2(10.0, 200.0));
        assert_eq!(FADE_AT_REST.at(rect, 300.0), 0.3);
        assert_eq!(FADE_AT_REST.at(rect, 100.0), 0.0);
        assert_eq!(FADE_IN_USE.at(rect, 200.0), 0.55);
        assert_eq!(FADE_AT_REST.lerp(FADE_IN_USE, 1.0).bottom, 0.8);
    }
}
