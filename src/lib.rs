//! A compact, on-demand command palette following Moon's interaction model.

mod rows;
mod style;

use egui::{vec2, Align2, CornerRadius, Key, RichText, Stroke};

use rows::{draw_row, rows_height};

pub use rows::{cut_to_width, paint_highlight, Kept, Row};
pub use style::PaletteStyle;

const ROWS_HEIGHT_OF_SCREEN: f32 = 0.55;

/// The gap between the query line's edge and what is typed in it.
const LINE_PADDING: i8 = 7;

#[derive(Clone, Debug)]
pub struct Command {
    pub id: String,
    pub title: String,
    pub description: String,
    pub shortcut: String,
}

/// What the palette says around its rows, for the list it is given.
pub struct View<'a> {
    /// Greyed in the query line while nothing is typed.
    pub hint: &'a str,
    /// Said in place of the rows when there are none.
    pub empty: &'a str,
    /// A small note under the rows.
    pub footnote: Option<&'a str>,
    /// Which end of a row's second line is kept when it is too long for the row.
    pub description_kept: Kept,
}

/// Something drawn on the query line and under it: a switch at the end of the line, a box
/// for the scope of a search.
pub trait LineAccessory {
    /// How much room the query line leaves at its end for what `show` draws over it.
    fn room_at_end(&self, ui: &egui::Ui) -> f32;

    /// Drawn right after the query line, whose rect is given.
    fn show(&mut self, ui: &mut egui::Ui, line: egui::Rect);
}

impl LineAccessory for () {
    fn room_at_end(&self, _ui: &egui::Ui) -> f32 {
        0.0
    }

    fn show(&mut self, _ui: &mut egui::Ui, _line: egui::Rect) {}
}

pub struct CommandPalette {
    open: bool,
    query: String,
    highlighted: usize,
    highlight_query: String,
    rect: Option<egui::Rect>,
    select_query: bool,
    style: PaletteStyle,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self {
            open: false,
            query: String::new(),
            highlighted: 0,
            highlight_query: String::new(),
            rect: None,
            select_query: false,
            style: PaletteStyle::default(),
        }
    }
}

impl CommandPalette {
    /// Open it on an empty query, at the top of the list, and drawn nowhere yet.
    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.highlighted = 0;
        self.highlight_query.clear();
        self.rect = None;
        self.select_query = false;
    }

    /// Open it on a query to type over: it is selected, so the first key typed replaces it.
    pub fn open_on(&mut self, query: &str) {
        self.open();
        self.query = query.to_string();
        self.highlight_query.clone_from(&self.query);
        self.select_query = true;
    }

    /// Put it away. The rect goes with it so the next one it draws is the one clicks are
    /// measured against.
    pub fn dismiss(&mut self) {
        self.open = false;
        self.rect = None;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn rect(&self) -> Option<egui::Rect> {
        self.rect
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// The row the keyboard is on, by its place in the rows last shown.
    pub fn highlighted(&self) -> usize {
        self.highlighted
    }

    /// Back to the first row, as when the list underneath is a different one.
    pub fn highlight_first(&mut self) {
        self.highlighted = 0;
    }

    pub fn style_mut(&mut self) -> &mut PaletteStyle {
        &mut self.style
    }

    /// Whether a press away from the palette or the Escape key asks for it to be put away.
    /// Answered before anything is drawn: the query line asks for the keyboard every frame it
    /// exists, so a palette still on screen would take it straight back off whatever was just
    /// clicked.
    pub fn dismissal_requested(&self, ctx: &egui::Context) -> bool {
        pressed_outside(ctx, self.rect) || ctx.input_mut(|input| input.key_pressed(Key::Escape))
    }

    /// The same as [`CommandPalette::show_rows`] over `commands` whose title or description
    /// holds every word typed. The id of the one chosen.
    pub fn show(&mut self, ctx: &egui::Context, commands: &[Command]) -> Option<String> {
        if !self.open {
            return None;
        }
        let matches = matching_indices(commands, &self.query);
        let rows: Vec<Row> = matches
            .iter()
            .map(|index| Row {
                title: commands[*index].title.clone(),
                description: commands[*index].description.clone(),
                shortcut: commands[*index].shortcut.clone(),
            })
            .collect();
        let view = View {
            hint: "Execute a command…",
            empty: "nothing matches",
            footnote: None,
            description_kept: Kept::Start,
        };
        self.show_rows(ctx, &rows, &view, &mut ())
            .map(|row| commands[matches[row]].id.clone())
    }

    /// Draw the palette over `rows`, which are listed as given: the caller has already
    /// searched them for what is typed. The place in `rows` of the one chosen.
    pub fn show_rows(
        &mut self,
        ctx: &egui::Context,
        rows: &[Row],
        view: &View,
        accessory: &mut impl LineAccessory,
    ) -> Option<usize> {
        if !self.open {
            return None;
        }
        if self.dismissal_requested(ctx) {
            self.dismiss();
            return None;
        }

        let (move_down, move_up, accept) = ctx.input_mut(|input| {
            (
                input.key_pressed(Key::ArrowDown),
                input.key_pressed(Key::ArrowUp),
                input.key_pressed(Key::Enter),
            )
        });
        // Typed since the highlight was picked: the list underneath it is a different list,
        // and the first match of the new one is what Enter runs.
        let retyped = self.highlight_query != self.query;
        if retyped {
            self.highlighted = 0;
            self.highlight_query.clone_from(&self.query);
        }
        if !rows.is_empty() {
            let last = rows.len() - 1;
            if move_down {
                self.highlighted = (self.highlighted + 1).min(last);
            }
            if move_up {
                self.highlighted = self.highlighted.saturating_sub(1);
            }
            self.highlighted = self.highlighted.min(last);
        }

        let mut chosen = accept
            .then_some(self.highlighted)
            .filter(|_| !rows.is_empty());
        let screen = ctx.viewport_rect();
        let area = egui::Area::new("command-palette".into())
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_TOP, vec2(0.0, screen.height() * 0.12))
            .show(ctx, |ui| {
                frame(&self.style).show(ui, |ui| {
                    ui.set_width(frame_width(screen));
                    // The line makes room at its end for the accessory: what is typed stops
                    // short of it rather than running under it.
                    let line_end = i8::try_from(
                        (f32::from(LINE_PADDING) + accessory.room_at_end(ui)).ceil() as i32,
                    )
                    .expect("the line's accessory is wider than a line's margin can be");
                    let entry = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text(view.hint)
                            .desired_width(f32::INFINITY)
                            .margin(egui::Margin {
                                left: LINE_PADDING,
                                right: line_end,
                                top: 5,
                                bottom: 5,
                            }),
                    );
                    entry.request_focus();
                    if std::mem::take(&mut self.select_query) {
                        select_all(ui.ctx(), entry.id, &self.query);
                    }
                    accessory.show(ui, entry.rect);

                    ui.add_space(6.0);
                    if rows.is_empty() {
                        ui.label(RichText::new(view.empty).color(self.style.muted));
                        return;
                    }

                    // The rows scroll rather than the box growing: a list taller than the
                    // window would be pushed up to fit, search line and all, every time the
                    // matches came in. The height is the rows' own, capped - an area hands
                    // its contents last frame's rect to fit in, so a scroll area left to size
                    // itself would stay as short as the empty palette was. The highlight is
                    // kept in view when the keyboard moves it; a pointer over a row is
                    // already looking at it.
                    let keep_highlight_in_view = move_down || move_up || retyped;
                    let height = rows_height(
                        rows.len(),
                        self.style.row_height,
                        ui.spacing().item_spacing.y,
                    )
                    .min(screen.height() * ROWS_HEIGHT_OF_SCREEN);
                    egui::ScrollArea::vertical()
                        .id_salt("command-palette-results")
                        .max_height(height)
                        .min_scrolled_height(height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (index, row) in rows.iter().enumerate() {
                                let highlighted = index == self.highlighted;
                                let response = draw_row(
                                    ui,
                                    row,
                                    view.description_kept,
                                    highlighted,
                                    &self.style,
                                );
                                if highlighted && keep_highlight_in_view {
                                    response.scroll_to_me(None);
                                }
                                if response.clicked() {
                                    chosen = Some(index);
                                }
                                if response.hovered() {
                                    self.highlighted = index;
                                }
                            }
                        });
                    if let Some(footnote) = view.footnote {
                        ui.label(
                            RichText::new(footnote)
                                .size(self.style.detail_size)
                                .color(self.style.muted),
                        );
                    }
                });
            });
        self.rect = Some(area.response.rect);

        if chosen.is_some() {
            self.dismiss();
        }
        chosen
    }
}

/// The box the palette is drawn in, floating over the window: the panel's ground, a hairline
/// round it and a shadow under it.
pub fn frame(style: &PaletteStyle) -> egui::Frame {
    egui::Frame::new()
        .fill(style.panel)
        .stroke(Stroke::new(1.0, style.line))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(9))
        .shadow(egui::epaint::Shadow {
            offset: [0, 10],
            blur: 28,
            spread: 0,
            color: style.shadow,
        })
}

/// How wide that box is: half the window, within what a line of a command and a phone's
/// window allow.
pub fn frame_width(screen: egui::Rect) -> f32 {
    (screen.width() * 0.5)
        .clamp(360.0, 560.0)
        .min((screen.width() - 36.0).max(120.0))
}

/// Whether a pointer button went down this frame away from where the palette drew last frame.
/// Before it has drawn once there is nowhere to be outside of, and the press is somebody
/// else's business.
pub fn pressed_outside(ctx: &egui::Context, drawn_at: Option<egui::Rect>) -> bool {
    let Some(drawn_at) = drawn_at else {
        return false;
    };
    ctx.input(|input| {
        input.pointer.any_pressed()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|at| !drawn_at.contains(at))
    })
}

/// Select the whole of the palette's line.
fn select_all(ctx: &egui::Context, id: egui::Id, query: &str) {
    let Some(mut state) = egui::TextEdit::load_state(ctx, id) else {
        return;
    };
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(0),
            egui::text::CCursor::new(query.chars().count()),
        )));
    state.store(ctx, id);
}

fn matching_indices(commands: &[Command], query: &str) -> Vec<usize> {
    let terms: Vec<String> = query
        .trim()
        .to_lowercase()
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect();
    commands
        .iter()
        .enumerate()
        .filter(|(_, command)| {
            let searchable = format!("{} {}", command.title, command.description).to_lowercase();
            terms.iter().all(|term| searchable.contains(term))
        })
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_term_must_match_title_or_description() {
        let commands = vec![Command {
            id: "sphere".into(),
            title: "Add Sphere".into(),
            description: "Create a round primitive".into(),
            shortcut: String::new(),
        }];
        assert_eq!(matching_indices(&commands, "sphere round"), vec![0]);
        assert!(matching_indices(&commands, "sphere cube").is_empty());
    }

    #[test]
    fn three_rows_want_the_gaps_between_them() {
        assert_eq!(rows_height(3, 34.0, 4.0), 3.0 * 34.0 + 8.0);
        assert_eq!(rows_height(1, 34.0, 4.0), 34.0);
        assert_eq!(rows_height(0, 34.0, 4.0), 0.0);
    }
}
