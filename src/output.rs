//! Output box - what something run from the palette printed, in a box of its own under the
//! palette: a bar naming what ran, with a spinner while it runs and a × to close it, over its
//! lines, which scroll once there are more than [`LINES_SHOWN`] and stay on the newest.

use egui::{vec2, Color32, RichText};

use crate::{frame, CommandPalette};

/// The gap between the palette and the box under it.
const GAP: f32 = 8.0;

/// How many lines the box shows before it scrolls.
const LINES_SHOWN: f32 = 10.0;

/// How wide the spinner of something still running is.
const SPINNER_SIZE: f32 = 9.0;

/// One line printed, in the colour it is told apart by - `None` is the style's text.
#[derive(Clone, Debug)]
pub struct OutputLine {
    pub text: String,
    pub ink: Option<Color32>,
}

/// What the box shows.
pub struct OutputBox<'a> {
    /// What ran, written in the bar.
    pub title: &'a str,
    /// Whether it is still running, which a spinner in the bar says.
    pub busy: bool,
    pub lines: &'a [OutputLine],
    /// Draws the button that closes the box, at the end of the bar: the caller's own close
    /// mark, so the box closes the way everything else in its window does.
    pub close_button: &'a dyn Fn(&mut egui::Ui) -> egui::Response,
}

impl CommandPalette {
    /// Draw `output` under the palette, which has to have been drawn this frame. Whether its ×
    /// was clicked.
    pub fn show_output(&mut self, ctx: &egui::Context, output: &OutputBox) -> bool {
        let Some(palette) = self.rect else {
            return false;
        };
        let style = self.style.clone();
        let mono = egui::FontId::monospace(style.detail_size + 1.0);
        let mut closed = false;
        let area = egui::Area::new("command-palette-output".into())
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(palette.min.x, palette.max.y + GAP))
            .show(ctx, |ui| {
                frame(&style).show(ui, |ui| {
                    // The frame's width is the palette's, its margins in it.
                    ui.set_width(
                        palette.width() - 2.0 * f32::from(frame(&style).inner_margin.left),
                    );
                    ui.horizontal(|ui| {
                        let close = ui
                            .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let close = (output.close_button)(ui);
                                if output.busy {
                                    ui.add(
                                        egui::Spinner::new().size(SPINNER_SIZE).color(style.accent),
                                    );
                                }
                                // What ran, from the left, in the room the two leave.
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(output.title)
                                                    .font(mono.clone())
                                                    .color(style.muted),
                                            )
                                            .truncate(),
                                        );
                                    },
                                );
                                close
                            })
                            .inner;
                        closed = close.clicked();
                    });
                    ui.separator();
                    let line_height = ui.fonts_mut(|fonts| fonts.row_height(&mono));
                    let gap = ui.spacing().item_spacing.y;
                    egui::ScrollArea::vertical()
                        .id_salt("command-palette-output-lines")
                        .max_height(LINES_SHOWN * (line_height + gap))
                        .auto_shrink([false, true])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = vec2(0.0, gap);
                            for line in output.lines {
                                ui.label(
                                    RichText::new(&line.text)
                                        .font(mono.clone())
                                        .color(line.ink.unwrap_or(style.text)),
                                );
                            }
                        });
                });
            });
        self.note_beside(area.response.rect);
        closed
    }
}
