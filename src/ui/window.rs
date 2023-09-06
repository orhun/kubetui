use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Tabs},
    Frame,
};

use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

use crate::{
    event::{kubernetes::Kube, UserEvent},
    logger,
};

use super::{
    event::{EventResult, InnerCallback},
    popup::Popup,
    theme::UIStyle,
    util::{key_event_to_code, MousePosition, RectContainsPoint},
    widget::{Widget, WidgetTrait},
    Tab,
};

type HeaderCallback = Rc<dyn Fn() -> Paragraph<'static>>;

#[derive(Default)]
pub struct Window<'a> {
    tabs: Vec<Tab<'a>>,
    active_tab_index: usize,
    mouse_over_tab_index: Option<usize>,
    layout: Layout,
    chunk: Rect,
    callbacks: Vec<(UserEvent, InnerCallback)>,
    popups: Vec<Popup<'a>>,
    open_popup_id: Option<String>,
    header: Option<Header<'a>>,
    layout_index: WindowLayoutIndex,
    last_known_size: Rect,
    theme: WindowTheme,
}

#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct WindowTheme {
    pub tab: TabTheme,
    pub header: UIStyle,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct TabTheme {
    pub base: UIStyle,
    pub active: UIStyle,
    pub mouse_over: UIStyle,
}

impl Default for TabTheme {
    fn default() -> Self {
        Self {
            base: UIStyle::default(),
            active: UIStyle {
                fg: None,
                bg: None,
                modifier: Some(Modifier::REVERSED),
            },
            mouse_over: UIStyle {
                fg: Some(Color::DarkGray),
                bg: None,
                modifier: Some(Modifier::REVERSED),
            },
        }
    }
}

impl TabTheme {
    pub fn active_style(&self) -> Style {
        self.active.to_style()
    }
    pub fn mouse_over_style(&self) -> Style {
        self.mouse_over.to_style()
    }
}

#[derive(Default)]
struct WindowLayoutIndex {
    tab: usize,
    header: usize,
    contents: usize,
}

pub enum HeaderContent<'a> {
    Static(Vec<Line<'a>>),
    Callback(HeaderCallback),
}

impl Default for HeaderContent<'_> {
    fn default() -> Self {
        HeaderContent::Static(Default::default())
    }
}

#[derive(Default)]
pub struct Header<'a> {
    height: u16,
    content: HeaderContent<'a>,
}

impl<'a> Header<'a> {
    pub fn new_static(height: u16, content: Vec<Line<'a>>) -> Self {
        debug_assert!(0 < height, "Header height must be greater than 0");

        Self {
            height,
            content: HeaderContent::Static(content),
        }
    }

    pub fn new_callback<F>(height: u16, callback: F) -> Self
    where
        F: Fn() -> Paragraph<'static> + 'static,
    {
        debug_assert!(0 < height, "Header height must be greater than 0");

        Self {
            height,
            content: HeaderContent::Callback(Rc::new(callback)),
        }
    }

    pub fn content_update(&mut self, content: HeaderContent<'a>) {
        self.content = content;
    }
}

#[derive(Default)]
pub struct WindowBuilder<'a> {
    tabs: Vec<Tab<'a>>,
    callbacks: Vec<(UserEvent, InnerCallback)>,
    popups: Vec<Popup<'a>>,
    header: Option<Header<'a>>,
    theme: WindowTheme,
}

impl<'a> WindowBuilder<'a> {
    pub fn tabs(mut self, tabs: impl Into<Vec<Tab<'a>>>) -> Self {
        self.tabs = tabs.into();
        self
    }

    pub fn action<F, E: Into<UserEvent>>(mut self, ev: E, cb: F) -> Self
    where
        F: Fn(&mut Window) -> EventResult + 'static,
    {
        self.callbacks.push((ev.into(), Rc::new(cb)));
        self
    }

    pub fn popup(mut self, popup: impl Into<Vec<Popup<'a>>>) -> Self {
        self.popups = popup.into();
        self
    }

    pub fn header(mut self, header: Header<'a>) -> Self {
        self.header = Some(header);
        self
    }

    pub fn theme(mut self, theme: WindowTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn build(self) -> Window<'a> {
        let (layout_index, constraints) = if let Some(header) = &self.header {
            (
                WindowLayoutIndex {
                    tab: 0,
                    header: 2,
                    contents: 3,
                },
                vec![
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(header.height),
                    Constraint::Min(1),
                ],
            )
        } else {
            (
                WindowLayoutIndex {
                    tab: 0,
                    header: 0,
                    contents: 2,
                },
                vec![
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(1),
                ],
            )
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints);

        Window {
            tabs: self.tabs,
            layout,
            callbacks: self.callbacks,
            popups: self.popups,
            header: self.header,
            theme: self.theme,
            layout_index,
            ..Default::default()
        }
    }
}

// Window
impl<'a> Window<'a> {
    pub fn builder() -> WindowBuilder<'a> {
        WindowBuilder::default()
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let chunks = self.layout.split(chunk);

        let contents_index = self.layout_index.contents;
        self.tabs.iter_mut().for_each(|tab| {
            tab.update_chunk(chunks[contents_index]);
        });

        self.popups.iter_mut().for_each(|w| w.update_chunk(chunk))
    }

    fn chunks(&self) -> Rc<[Rect]> {
        self.layout.split(self.chunk)
    }

    pub fn widget(&self) -> Tabs {
        let titles: Vec<Line> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(tab_index, tab)| {
                if self
                    .mouse_over_tab_index
                    .is_some_and(|index| index == tab_index && index != self.active_tab_index)
                {
                    Line::from(Span::styled(
                        Self::tab_title_format(tab_index, tab.title()),
                        self.theme.tab.mouse_over_style(),
                    ))
                } else {
                    Line::from(Self::tab_title_format(tab_index, tab.title()))
                }
            })
            .collect();

        Tabs::new(titles)
            .block(self.tab_block())
            .select(self.active_tab_index)
            .highlight_style(self.theme.tab.active_style())
    }

    pub fn match_callback(&self, ev: UserEvent) -> Option<InnerCallback> {
        self.callbacks.iter().find_map(|(cb_ev, cb)| {
            logger!(debug, "match_callback {:?} <=> {:?}", ev, cb_ev);

            if *cb_ev == ev {
                Some(cb.clone())
            } else {
                None
            }
        })
    }

    pub fn update_header(&mut self, content: HeaderContent<'a>) {
        if let Some(h) = self.header.as_mut() {
            h.content_update(content);
        }
    }
}

// Popup
impl<'a> Window<'a> {
    pub fn open_popup(&mut self, id: impl Into<String>) {
        self.open_popup_id = Some(id.into());
    }

    pub fn close_popup(&mut self) {
        self.open_popup_id = None;
    }

    pub fn opening_popup(&self) -> bool {
        self.open_popup_id.is_some()
    }
}

// Tab
impl<'a> Window<'a> {
    pub fn active_tab_id(&self) -> &str {
        self.tabs[self.active_tab_index].id()
    }

    pub fn active_tab(&self) -> &Tab<'a> {
        &self.tabs[self.active_tab_index]
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab<'a> {
        &mut self.tabs[self.active_tab_index]
    }

    pub fn activate_tab_by_index(&mut self, index: usize) {
        if self.tabs.get(index).is_some() {
            self.active_tab_index = index;
        }
    }

    pub fn activate_next_tab(&mut self) {
        self.active_tab_index = (self.active_tab_index + 1) % self.tabs.len();
    }

    pub fn activate_prev_tab(&mut self) {
        self.active_tab_index = (self.active_tab_index + self.tabs.len() - 1) % self.tabs.len();
    }

    fn tab_title_format(index: usize, title: &str) -> String {
        format!("{}: {} ", index + 1, title)
    }

    fn tab_block(&self) -> Block<'a> {
        Block::default().style(self.theme.tab.base.to_style())
    }

    pub fn tab_chunk(&self) -> Rect {
        self.chunks()[self.layout_index.tab]
    }
}

// Pane
impl<'a> Window<'a> {
    pub fn find_widget(&self, id: &str) -> &Widget<'a> {
        if let Some(w) = self.popups.iter().find(|w| w.id() == id) {
            w.widget()
        } else {
            self.tabs
                .iter()
                .find_map(|t| t.find_widget(id))
                .unwrap_or_else(|| panic!("Could not find widget id [{}]", id))
        }
    }

    pub fn find_widget_mut(&mut self, id: &str) -> &mut Widget<'a> {
        if let Some(w) = self.popups.iter_mut().find(|w| w.id() == id) {
            w.widget_mut()
        } else {
            self.tabs
                .iter_mut()
                .find_map(|t| t.find_widget_mut(id))
                .unwrap_or_else(|| panic!("Could not find widget id [{}]", id))
        }
    }

    pub fn active_widget_id(&self) -> &str {
        self.active_tab().active_widget_id()
    }

    fn activate_next_widget(&mut self) {
        self.active_tab_mut().activate_next_widget();
    }

    fn activate_prev_widget(&mut self) {
        self.active_tab_mut().activate_prev_widget();
    }

    pub fn widget_clear(&mut self, id: &str) {
        self.find_widget_mut(id).clear();
    }

    pub fn activate_widget_by_id(&mut self, id: &str) {
        self.active_tab_mut().activate_widget_by_id(id)
    }

    pub fn clear_mouse_over(&mut self) {
        self.mouse_over_tab_index = None;

        self.active_tab_mut().clear_mouse_over();

        if let Some(id) = &self.open_popup_id {
            if let Some(Widget::MultipleSelect(w)) = self
                .popups
                .iter_mut()
                .find(|w| w.id() == id)
                .map(|w| w.widget_mut())
            {
                w.clear_mouse_over();
            }
        }
    }
}

// Render
impl<'a> Window<'a> {
    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        let size = f.size();

        if self.last_known_size != size {
            self.update_chunks(size);

            self.last_known_size = size;
        }

        self.render_tab(f);

        self.render_header(f);

        self.render_contents(f);

        self.render_popup(f);
    }

    fn render_tab<B: Backend>(&mut self, f: &mut Frame<B>) {
        f.render_widget(self.widget(), self.tab_chunk());
    }

    fn render_header<B: Backend>(&self, f: &mut Frame<B>) {
        if let Some(header) = &self.header {
            let w = match &header.content {
                HeaderContent::Static(content) => Paragraph::new(content.to_vec()),
                HeaderContent::Callback(callback) => (callback)(),
            };
            f.render_widget(
                w.style(self.theme.header.to_style()),
                self.chunks()[self.layout_index.header],
            );
        }
    }

    fn render_contents<B: Backend>(&mut self, f: &mut Frame<B>) {
        self.active_tab_mut().render(f);
    }

    fn render_popup<B: Backend>(&mut self, f: &mut Frame<B>) {
        if let Some(id) = &self.open_popup_id {
            if let Some(popup) = self.popups.iter_mut().find(|p| p.id() == id) {
                popup.render(f);
            }
        }
    }
}

enum AreaKind {
    Tab,
    Widgets,
    OutSide,
}

pub enum WindowEvent {
    CloseWindow,
    Continue,
    UpdateContents(Kube),
}

// Event
impl Window<'_> {
    pub fn on_event(&mut self, ev: UserEvent) -> EventResult {
        match ev {
            UserEvent::Key(ev) => self.on_key_event(ev),
            UserEvent::Mouse(ev) => self.on_mouse_event(ev),
            UserEvent::FocusLost => {
                self.clear_mouse_over();
                EventResult::Nop
            }
            UserEvent::FocusGained => {
                self.clear_mouse_over();

                EventResult::Nop
            }
        }
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.clear_mouse_over();

        if let Some(id) = &self.open_popup_id {
            if let Some(popup) = self.popups.iter_mut().find(|w| w.id() == id) {
                return popup.on_key_event(ev);
            }
        }

        let active_tab = self.active_tab_mut().active_widget_mut();

        match active_tab.on_key_event(ev) {
            EventResult::Ignore => match key_event_to_code(ev) {
                KeyCode::Tab => {
                    self.activate_next_widget();
                }

                KeyCode::BackTab => {
                    self.activate_prev_widget();
                }

                KeyCode::Char(n @ '1'..='9') => {
                    let index = n as usize - b'0' as usize;
                    self.activate_tab_by_index(index - 1);
                }

                _ => {
                    return EventResult::Ignore;
                }
            },
            ev => {
                return ev;
            }
        }

        EventResult::Nop
    }

    fn area_kind_by_cursor_position(&self, pos: (u16, u16)) -> AreaKind {
        if self.tab_chunk().contains_point(pos) {
            AreaKind::Tab
        } else if self.chunks()[self.layout_index.contents].contains_point(pos) {
            AreaKind::Widgets
        } else {
            AreaKind::OutSide
        }
    }

    pub fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        if let Some(id) = &self.open_popup_id {
            if let Some(popup) = self.popups.iter_mut().find(|w| w.id() == id) {
                if popup.chunk().contains_point(ev.position()) {
                    return popup.on_mouse_event(ev);
                } else if let MouseEventKind::Down(MouseButton::Left) = ev.kind {
                    self.close_popup();
                    return EventResult::Nop;
                }
            }
        }

        let pos = (ev.column, ev.row);

        let result = match self.area_kind_by_cursor_position(pos) {
            AreaKind::Tab => {
                self.on_tab_area_mouse_event(ev);

                EventResult::Nop
            }
            AreaKind::Widgets => {
                self.clear_mouse_over();

                self.active_tab_mut().on_mouse_event(ev)
            }
            AreaKind::OutSide => {
                self.clear_mouse_over();

                EventResult::Ignore
            }
        };

        result
    }

    fn on_tab_area_mouse_event(&mut self, ev: MouseEvent) {
        let pos = ev.position();

        let chunk = self.tab_block().inner(self.tab_chunk());
        let divider_width = 1;

        let mut x = chunk.left();
        let y = chunk.top();
        let h = chunk.height;

        for (i, tab) in self.tabs.iter().enumerate() {
            let w = Self::tab_title_format(i, tab.title()).width() as u16;
            x = x.saturating_add(1);

            let title_chunk = Rect::new(x, y, w, h);

            match ev.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if title_chunk.contains_point(pos) {
                        self.activate_tab_by_index(i);
                        break;
                    }
                }
                MouseEventKind::Moved => {
                    if title_chunk.contains_point(pos) {
                        self.mouse_over_tab_index = Some(i);
                        break;
                    }
                }
                _ => {}
            }

            x = x
                .saturating_add(1)
                .saturating_add(w)
                .saturating_add(divider_width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod ui {
        use super::*;
        use serde_yaml_0_9 as serde_yaml;

        mod tab_theme {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn deserialize_empty() {
                let actual: TabTheme = serde_yaml::from_str("").unwrap();

                let expected = TabTheme::default();

                println!("{:#?}", expected);

                assert_eq!(actual, expected)
            }
        }
    }
}
