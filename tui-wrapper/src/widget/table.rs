use crate::{
    event::{Callback, EventResult},
    key_event_to_code,
    util::{default_focus_block, focus_block},
    Window,
};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use derivative::*;

use std::rc::Rc;
use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Table as TTable, TableState},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use super::spans::generate_spans_line;
use super::wrap::wrap_line;
use super::{RenderTrait, WidgetItem, WidgetTrait};

const COLUMN_SPACING: u16 = 3;
const HIGHLIGHT_SYMBOL: &str = " ";
const ROW_START_INDEX: usize = 2;

type InnerCallback = Rc<dyn Fn(&mut Window, &[String]) -> EventResult>;

#[derive(Debug, Default)]
pub struct TableBuilder {
    id: String,
    title: String,
    header: Vec<String>,
    items: Vec<Vec<String>>,
}

impl TableBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn items(mut self, items: impl Into<Vec<Vec<String>>>) -> Self {
        self.items = items.into();
        self
    }

    pub fn header(mut self, header: impl Into<Vec<String>>) -> Self {
        self.header = header.into();
        self
    }

    pub fn build(self) -> Table<'static> {
        let table = Table {
            id: self.id,
            title: self.title,
            ..Default::default()
        };

        let mut table = table.header(self.header);

        table.set_items(WidgetItem::DoubleArray(self.items));
        table
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Table<'a> {
    id: String,
    title: String,
    chunk_index: usize,
    items: Vec<Vec<String>>,
    header: Vec<String>,
    header_row: Row<'a>,
    state: TableState,
    rows: Vec<Row<'a>>,
    widths: Vec<Constraint>,
    row_width: usize,
    digits: Vec<usize>,
    chunk: Rect,
    inner_chunk: Rect,
    row_bounds: Vec<(usize, usize)>,
    #[derivative(Debug = "ignore")]
    on_select: Option<InnerCallback>,
}

impl<'a> Table<'a> {
    pub fn header(mut self, header: impl Into<Vec<String>>) -> Self {
        self.header = header.into();

        let header_cells = self
            .header
            .iter()
            .cloned()
            .map(|h| Cell::from(h).style(Style::default().fg(Color::DarkGray)));

        self.header_row = Row::new(header_cells).bottom_margin(1);

        self.set_widths();
        self.set_rows();

        self
    }

    pub fn items(&self) -> &Vec<Vec<String>> {
        &self.items
    }

    pub fn state(&self) -> &TableState {
        &self.state
    }

    fn set_widths(&mut self) {
        if self.items.is_empty() {
            return;
        }

        self.digits = if self.header.is_empty() {
            self.items[0].iter().map(|i| i.width()).collect()
        } else {
            self.header.iter().map(|h| h.width()).collect()
        };

        for row in &self.items {
            for (i, col) in row.iter().enumerate() {
                let len = col.len();
                if self.digits.len() < i {
                    break;
                }

                if self.digits[i] < len {
                    self.digits[i] = len
                }
            }
        }

        if self.digits.iter().sum::<usize>()
            + (COLUMN_SPACING as usize * self.digits.len().saturating_sub(1))
            <= self.row_width
        {
            self.widths = self
                .digits
                .iter()
                .map(|d| Constraint::Length(*d as u16))
                .collect()
        } else {
            self.digits[0] = self.row_width.saturating_sub(
                (COLUMN_SPACING as usize * self.digits.len().saturating_sub(1))
                    + self.digits.iter().skip(1).sum::<usize>(),
            );

            self.widths = self
                .digits
                .iter()
                .map(|d| Constraint::Length(*d as u16))
                .collect();
        }
    }

    fn set_rows(&mut self) {
        if self.digits.is_empty() {
            return;
        }

        let mut margin = 0;
        let mut row_bounds: Vec<(usize, usize)> = Vec::new();

        self.rows = self
            .items
            .iter()
            .scan(0, |current_height, row| {
                let cells: Vec<(Cell, usize)> = row
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, cell)| {
                        let wrapped = wrap_line(&cell, self.digits[i]);

                        let mut height = 1;
                        if height < wrapped.len() {
                            height = wrapped.len();
                            margin = 1;
                        }

                        (Cell::from(generate_spans_line(&wrapped)), height)
                    })
                    .collect();

                let height = if let Some((_, h)) = cells.iter().max_by_key(|(_, h)| h) {
                    *h
                } else {
                    1
                };

                row_bounds.push((*current_height, *current_height + height.saturating_sub(1)));
                *current_height += height;

                let cells = cells.into_iter().map(|(c, _)| c);
                Some(Row::new(cells).height(height as u16))
            })
            .collect();

        self.row_bounds = row_bounds;

        if margin == 1 {
            self.rows = self
                .rows
                .iter()
                .cloned()
                .map(|row| row.bottom_margin(margin))
                .collect();

            self.row_bounds = self
                .row_bounds
                .iter()
                .scan(0, |height, b| {
                    let b = (b.0 + *height, b.1 + *height);

                    *height += 1;
                    Some(b)
                })
                .collect();
        }
    }
}

impl WidgetTrait for Table<'_> {
    fn focusable(&self) -> bool {
        true
    }

    fn select_next(&mut self, index: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.len().saturating_sub(1) <= i + index {
                    self.items.len().saturating_sub(1)
                } else {
                    i + index
                }
            }
            None => 0,
        };

        self.state.select(Some(i));
    }

    fn select_prev(&mut self, index: usize) {
        let i = self.state.selected().unwrap_or(0);

        self.state.select(Some(i.saturating_sub(index)));
    }

    fn select_first(&mut self) {
        self.state.select(Some(0))
    }

    fn select_last(&mut self) {
        if self.items.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(Some(self.items.len() - 1))
        }
    }

    fn set_items(&mut self, items: WidgetItem) {
        let items = items.double_array();

        match items.len() {
            0 => self.state.select(None),
            len if len < self.items.len() => self.state.select(Some(len - 1)),
            _ => {
                if self.state.selected() == None {
                    self.state.select(Some(0))
                }
            }
        }

        self.items = items;
        self.set_widths();
        self.set_rows();
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
        self.inner_chunk = default_focus_block().inner(chunk);
        self.row_width = self.inner_chunk.width.saturating_sub(2) as usize;
    }

    fn clear(&mut self) {
        *self = Self::default();
    }

    fn get_item(&self) -> Option<WidgetItem> {
        self.state
            .selected()
            .map(|i| WidgetItem::Array(self.items[i].clone()))
    }

    fn append_items(&mut self, _: WidgetItem) {
        todo!()
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        if self.items.is_empty() {
            return EventResult::Nop;
        }

        let (_, row) = (
            ev.column.saturating_sub(self.inner_chunk.left()) as usize,
            ev.row.saturating_sub(self.inner_chunk.top()) as usize,
        );

        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let offset = self.state.offset();
                let offset_bound = self.row_bounds[offset];
                if let Some((index, _)) =
                    self.row_bounds[offset..].iter().enumerate().find(|(_, b)| {
                        let b = (
                            b.0 - offset_bound.0 + ROW_START_INDEX,
                            b.1 - offset_bound.1 + ROW_START_INDEX,
                        );

                        b.0 <= row && row <= b.1
                    })
                {
                    self.state.select(Some(index + offset));
                }

                return EventResult::Callback(self.on_select_callback());
            }

            MouseEventKind::ScrollDown => {
                self.select_next(1);
                return EventResult::Nop;
            }
            MouseEventKind::ScrollUp => {
                self.select_prev(1);
                return EventResult::Nop;
            }
            _ => {}
        }

        EventResult::Ignore
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match key_event_to_code(ev) {
            KeyCode::Char('j') | KeyCode::Down | KeyCode::PageDown => {
                self.select_next(1);
            }

            KeyCode::Char('k') | KeyCode::Up | KeyCode::PageUp => {
                self.select_prev(1);
            }

            KeyCode::Char('G') | KeyCode::End => {
                self.select_last();
            }

            KeyCode::Char('g') | KeyCode::Home => {
                self.select_first();
            }

            KeyCode::Enter => {
                return EventResult::Callback(self.on_select_callback());
            }

            KeyCode::Char(_) => {
                return EventResult::Ignore;
            }

            _ => {
                return EventResult::Ignore;
            }
        }

        EventResult::Nop
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn id(&self) -> &str {
        &self.id
    }
}

impl<'a> Table<'a> {
    pub fn on_select<F>(mut self, cb: F) -> Self
    where
        F: Fn(&mut Window, &[String]) -> EventResult + 'static,
    {
        self.on_select = Some(Rc::new(cb));
        self
    }

    fn on_select_callback(&self) -> Option<Callback> {
        self.on_select.clone().and_then(|cb| {
            self.selected_item()
                .map(|v| Callback::from_fn(move |w| cb(w, &v)))
        })
    }

    fn selected_item(&self) -> Option<Rc<Vec<String>>> {
        self.state
            .selected()
            .map(|i| Rc::new(self.items[i].clone()))
    }
}

impl RenderTrait for Table<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool)
    where
        B: Backend,
    {
        let title = self.title().to_string();
        let mut widget = TTable::new(self.rows.clone())
            .block(focus_block(&title, selected))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(HIGHLIGHT_SYMBOL)
            .column_spacing(COLUMN_SPACING)
            .widths(&self.widths);

        if !self.header.is_empty() {
            widget = widget.header(self.header_row.clone());
        }

        f.render_stateful_widget(widget, self.chunk, &mut self.state);
    }
}
