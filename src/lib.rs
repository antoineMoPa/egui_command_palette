//! A compact, on-demand command palette following Moon's interaction model.

mod history;
mod output;
mod rows;
mod sections;
mod style;

use egui::{vec2, Align2, CornerRadius, Key, RichText, Stroke};

use rows::draw_row;
use sections::{draw_header, draw_placeholder, sections_height};

pub use history::HistoryBox;
pub use output::{OutputBox, OutputLine};
pub use rows::{cut_to_width, paint_highlight, Aside, Choice, Kept, Row};
pub use sections::{Section, SectionHeader};
pub use style::PaletteStyle;

const ROWS_HEIGHT_OF_SCREEN: f32 = 0.55;

/// How far down the window the palette's top is: low enough to leave room for a history box
/// above it - see [`CommandPalette::show_history`] - so the palette is in the same place
/// with or without one.
const TOP_OF_SCREEN: f32 = 0.34;

/// The gap between the label at the start of the query line and what is typed after it.
const LABEL_GAP: f32 = 6.0;

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
    /// Written small in the accent colour at the start of the query line, before what is
    /// typed: what the line is typing into, when that is not the list under it.
    pub label: Option<&'a str>,
    /// Whether the row the keyboard is on is shown: it is not while the keyboard is in
    /// another list, the history box's.
    pub highlight_shown: bool,
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

/// What was done to the list on one frame of [`CommandPalette::show_sections`].
#[derive(Default)]
pub struct Shown {
    /// The row run, by its place among the rows of every section.
    pub chosen: Option<usize>,
    /// The section whose header's right side was clicked, by its place in the list.
    pub more_clicked: Option<usize>,
    /// A row's choice clicked, which runs the row with it: the row's place among the rows of
    /// every section, and the choice's among the row's - see [`Row::choices`].
    pub choice_clicked: Option<(usize, usize)>,
}

pub struct CommandPalette {
    open: bool,
    query: String,
    highlighted: usize,
    highlight_query: String,
    rect: Option<egui::Rect>,
    select_query: bool,
    /// Set by [`CommandPalette::set_query`], for the next frame to put the caret after it.
    caret_to_end: bool,
    style: PaletteStyle,
    /// The boxes drawn with the palette since it was last drawn - the history box, the output
    /// box: a press inside one is not a press away from the palette.
    beside: Vec<egui::Rect>,
    /// The row the history box last kept in view - see [`CommandPalette::show_history`].
    history_in_view: Option<usize>,
    /// The tallest the list has been since something was typed, which it does not go below
    /// until the query is emptied: a list that shrank and grew as each key changed what
    /// matched would move every row under the eye. The whole list shown for an empty query
    /// is not counted, or what is typed would sit over an empty box that tall.
    list_height_kept: f32,
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
            caret_to_end: false,
            style: PaletteStyle::default(),
            beside: Vec::new(),
            history_in_view: None,
            list_height_kept: 0.0,
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
        self.beside.clear();
        self.history_in_view = None;
        self.list_height_kept = 0.0;
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
        self.beside.clear();
    }

    /// Put `query` on the line in place of what is typed, with the caret at its end, as if it
    /// had been typed.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.caret_to_end = true;
    }

    /// Keep it open after a row was run - see [`Row::stays_open`] - or open it again, with
    /// what is typed and the row the keyboard is on as they were.
    pub fn keep_open(&mut self) {
        self.open = true;
    }

    /// Note a box drawn with the palette this frame - see `beside`.
    fn note_beside(&mut self, rect: egui::Rect) {
        self.beside.push(rect);
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

    /// Put the highlight on the row at `index` among the rows about to be shown: the same
    /// row as before, which the rows arriving above it have moved down. Typing since still
    /// puts it back on the first row.
    pub fn highlight(&mut self, index: usize) {
        self.highlighted = index;
    }

    pub fn style_mut(&mut self) -> &mut PaletteStyle {
        &mut self.style
    }

    /// Whether a press away from the palette or the Escape key asks for it to be put away.
    /// Answered before anything is drawn: the query line asks for the keyboard every frame it
    /// exists, so a palette still on screen would take it straight back off whatever was just
    /// clicked.
    pub fn dismissal_requested(&self, ctx: &egui::Context) -> bool {
        let pressed_beside = ctx.input(|input| {
            input.pointer.any_pressed()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|at| self.beside.iter().any(|rect| rect.contains(at)))
        });
        (pressed_outside(ctx, self.rect) && !pressed_beside)
            || ctx.input_mut(|input| input.key_pressed(Key::Escape))
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
                ..Row::default()
            })
            .collect();
        let view = View {
            hint: "Execute a command…",
            empty: "nothing matches",
            footnote: None,
            label: None,
            highlight_shown: true,
        };
        self.show_rows(ctx, &rows, Kept::Start, &view, &mut ())
            .map(|row| commands[matches[row]].id.clone())
    }

    /// Draw the palette over `rows`, which are listed as given: the caller has already
    /// searched them for what is typed. `description_kept` is which end of a row's second
    /// line is kept when it is too long for the row. The place in `rows` of the one chosen.
    pub fn show_rows(
        &mut self,
        ctx: &egui::Context,
        rows: &[Row],
        description_kept: Kept,
        view: &View,
        accessory: &mut impl LineAccessory,
    ) -> Option<usize> {
        let bare = Section {
            header: None,
            rows,
            description_kept,
            placeholders: 0,
        };
        self.show_sections(ctx, &[bare], view, accessory).chosen
    }

    /// The same over rows in sections, listed as given, one under the other. The keyboard
    /// walks the rows of every section as one list, skipping the headers and placeholders.
    pub fn show_sections(
        &mut self,
        ctx: &egui::Context,
        sections: &[Section],
        view: &View,
        accessory: &mut impl LineAccessory,
    ) -> Shown {
        if !self.open {
            return Shown::default();
        }
        if self.dismissal_requested(ctx) {
            self.dismiss();
            return Shown::default();
        }
        // Drawn again after this, if they still are.
        self.beside.clear();
        let row_count: usize = sections.iter().map(|section| section.rows.len()).sum();
        let placeholder_count: usize = sections.iter().map(|section| section.placeholders).sum();

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
        if row_count > 0 {
            let last = row_count - 1;
            if move_down {
                self.highlighted = (self.highlighted + 1).min(last);
            }
            if move_up {
                self.highlighted = self.highlighted.saturating_sub(1);
            }
            self.highlighted = self.highlighted.min(last);
        }

        let mut chosen = accept.then_some(self.highlighted).filter(|_| row_count > 0);
        let mut more_clicked = None;
        let mut choice_clicked = None;
        let screen = ctx.viewport_rect();
        let area = egui::Area::new("command-palette".into())
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_TOP, vec2(0.0, screen.height() * TOP_OF_SCREEN))
            .show(ctx, |ui| {
                frame(&self.style).show(ui, |ui| {
                    ui.set_width(frame_width(screen));
                    // The line makes room at its end for the accessory: what is typed stops
                    // short of it rather than running under it.
                    let line_end = i8::try_from(
                        (f32::from(LINE_PADDING) + accessory.room_at_end(ui)).ceil() as i32,
                    )
                    .expect("the line's accessory is wider than a line's margin can be");
                    // And at its start for the label, which is drawn in that room.
                    let label = view.label.map(|label| {
                        ui.painter().layout_no_wrap(
                            label.to_string(),
                            egui::FontId::proportional(self.style.detail_size),
                            self.style.accent,
                        )
                    });
                    let line_start = match &label {
                        Some(label) => i8::try_from(
                            (f32::from(LINE_PADDING) + label.size().x + LABEL_GAP).ceil() as i32,
                        )
                        .expect("the line's label is wider than a line's margin can be"),
                        None => LINE_PADDING,
                    };
                    let entry = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text(view.hint)
                            .desired_width(f32::INFINITY)
                            .margin(egui::Margin {
                                left: line_start,
                                right: line_end,
                                top: 5,
                                bottom: 5,
                            }),
                    );
                    entry.request_focus();
                    if std::mem::take(&mut self.select_query) {
                        select_all(ui.ctx(), entry.id, &self.query);
                    }
                    if std::mem::take(&mut self.caret_to_end) {
                        select_from(ui.ctx(), entry.id, &self.query, self.query.chars().count());
                    }
                    if let Some(label) = label {
                        let at = egui::pos2(
                            entry.rect.min.x + f32::from(LINE_PADDING),
                            entry.rect.center().y - label.size().y / 2.0,
                        );
                        ui.painter().galley(at, label, self.style.accent);
                    }
                    accessory.show(ui, entry.rect);

                    ui.add_space(6.0);
                    if row_count == 0 && placeholder_count == 0 {
                        let kept = self.list_height_kept;
                        ui.allocate_ui(vec2(ui.available_width(), kept), |ui| {
                            ui.set_min_height(kept);
                            ui.label(RichText::new(view.empty).color(self.style.muted));
                        });
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
                    // A row takes the highlight from the pointer only when it moved onto it:
                    // a pointer left resting over the list would otherwise take it back from
                    // the first row on every key typed, a frame after it was drawn there.
                    let pointer_moved = ui.input(|input| input.pointer.delta() != egui::Vec2::ZERO);
                    let gap = ui.spacing().item_spacing.y;
                    let typed = !self.query.trim().is_empty();
                    if !typed {
                        self.list_height_kept = 0.0;
                    }
                    let height = sections_height(sections, &self.style, gap)
                        .max(self.list_height_kept)
                        .min(screen.height() * ROWS_HEIGHT_OF_SCREEN);
                    if typed {
                        self.list_height_kept = height;
                    }
                    let style = &self.style;
                    let highlighted_now = &mut self.highlighted;
                    // The row's place among the rows of every section.
                    let mut index = 0;
                    let mut draw_sections = |ui: &mut egui::Ui, sections: &[Section]| {
                        for (at, section) in sections.iter().enumerate() {
                            if let Some(header) = &section.header {
                                if draw_header(ui, header, style) {
                                    more_clicked = Some(at);
                                }
                            }
                            for row in section.rows {
                                let highlighted = view.highlight_shown && index == *highlighted_now;
                                let (response, clicked) =
                                    draw_row(ui, row, section.description_kept, highlighted, style);
                                if highlighted && keep_highlight_in_view {
                                    response.scroll_to_me(None);
                                }
                                if response.clicked() {
                                    chosen = Some(index);
                                }
                                if let Some(choice) = clicked {
                                    choice_clicked = Some((index, choice));
                                }
                                if response.hovered() && pointer_moved {
                                    *highlighted_now = index;
                                }
                                index += 1;
                            }
                            for _ in 0..section.placeholders {
                                draw_placeholder(ui, style);
                            }
                        }
                    };
                    egui::ScrollArea::vertical()
                        .id_salt("command-palette-results")
                        .max_height(height)
                        .min_scrolled_height(height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| draw_sections(ui, sections));
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

        let stays_open = chosen.is_some_and(|index| {
            sections
                .iter()
                .flat_map(|section| section.rows)
                .nth(index)
                .expect("the row chosen is one of the rows")
                .stays_open
        });
        if (chosen.is_some() && !stays_open) || choice_clicked.is_some() {
            self.dismiss();
        }
        Shown {
            chosen,
            more_clicked,
            choice_clicked,
        }
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
    select_from(ctx, id, query, 0);
}

/// Select the palette's line from the character at `from` to its end: from its end is only
/// the caret, there.
fn select_from(ctx: &egui::Context, id: egui::Id, query: &str, from: usize) {
    let Some(mut state) = egui::TextEdit::load_state(ctx, id) else {
        return;
    };
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(from),
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
        let style = PaletteStyle {
            row_height: 34.0,
            header_height: 30.0,
            ..PaletteStyle::default()
        };
        let rows = vec![Row::default(); 3];
        let bare = |count: usize| Section {
            header: None,
            rows: &rows[..count],
            description_kept: Kept::Start,
            placeholders: 0,
        };
        assert_eq!(sections_height(&[bare(3)], &style, 4.0), 3.0 * 34.0 + 8.0);
        assert_eq!(sections_height(&[bare(1)], &style, 4.0), 34.0);
        assert_eq!(sections_height(&[bare(0)], &style, 4.0), 0.0);
    }

    /// A header and its placeholders take their room as rows do, gaps and all.
    #[test]
    fn headers_and_placeholders_count_in_the_height() {
        let style = PaletteStyle {
            row_height: 34.0,
            header_height: 30.0,
            ..PaletteStyle::default()
        };
        let rows = vec![Row::default(); 1];
        let section = Section {
            header: Some(SectionHeader {
                label: "files",
                count: "1…",
                busy: true,
                more: None,
            }),
            rows: &rows,
            description_kept: Kept::End,
            placeholders: 2,
        };
        assert_eq!(
            sections_height(&[section], &style, 4.0),
            30.0 + 3.0 * 34.0 + 3.0 * 4.0
        );
    }
}
