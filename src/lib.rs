use tui::style::{Modifier, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Paragraph, Widget};

#[derive(Clone, Copy, Debug)]
pub enum Key {
    Char(char),
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Tab,
    Delete,
    Home,
    End,
    Null,
}

#[derive(Debug)]
pub struct Input {
    pub key: Key,
    pub ctrl: bool,
}

impl Default for Input {
    fn default() -> Self {
        Input {
            key: Key::Null,
            ctrl: false,
        }
    }
}

impl From<crossterm::event::Event> for Input {
    fn from(event: crossterm::event::Event) -> Self {
        if let crossterm::event::Event::Key(key) = event {
            Self::from(key)
        } else {
            Self::default()
        }
    }
}

impl From<crossterm::event::KeyEvent> for Input {
    fn from(key: crossterm::event::KeyEvent) -> Self {
        use crossterm::event::{KeyCode, KeyModifiers};
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let key = match key.code {
            KeyCode::Char(c) => Key::Char(c),
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Enter => Key::Enter,
            KeyCode::Left => Key::Left,
            KeyCode::Right => Key::Right,
            KeyCode::Up => Key::Up,
            KeyCode::Down => Key::Down,
            KeyCode::Tab => Key::Tab,
            KeyCode::Delete => Key::Delete,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            _ => Key::Null,
        };
        Self { key, ctrl }
    }
}

pub struct TextArea<'a> {
    lines: Vec<String>,
    block: Option<Block<'a>>,
    style: Style,
    cursor: (usize, usize), // 0-base
    tab: &'a str,
}

impl<'a> Default for TextArea<'a> {
    fn default() -> Self {
        Self {
            lines: vec![" ".to_string()],
            block: None,
            style: Style::default(),
            cursor: (0, 0),
            tab: "    ",
        }
    }
}

impl<'a> TextArea<'a> {
    pub fn input(&mut self, input: impl Into<Input>) {
        let input = input.into();
        if input.ctrl {
            match input.key {
                Key::Char('h') => self.delete_char(),
                Key::Char('m') => self.insert_newline(),
                Key::Char('p') => self.cursor_up(),
                Key::Char('f') => self.cursor_forward(),
                Key::Char('n') => self.cursor_down(),
                Key::Char('b') => self.cursor_back(),
                Key::Char('a') => self.cursor_start(),
                Key::Char('e') => self.cursor_end(),
                _ => {}
            }
        } else {
            match input.key {
                Key::Char(c) => self.insert_char(c),
                Key::Backspace => self.delete_char(),
                Key::Tab => self.insert_tab(),
                Key::Enter => self.insert_newline(),
                Key::Up => self.cursor_up(),
                Key::Right => self.cursor_forward(),
                Key::Down => self.cursor_down(),
                Key::Left => self.cursor_back(),
                Key::Home => self.cursor_start(),
                Key::End => self.cursor_end(),
                _ => {}
            }
        }

        // Check invariants
        debug_assert!(!self.lines.is_empty(), "no line after {:?}", input);
        for (i, l) in self.lines.iter().enumerate() {
            debug_assert!(
                l.ends_with(' '),
                "line {} does not end with space after {:?}: {:?}",
                i + 1,
                input,
                l,
            );
        }
        let (r, c) = self.cursor;
        debug_assert!(
            self.lines.len() > r,
            "cursor {:?} exceeds max lines {} after {:?}",
            self.cursor,
            self.lines.len(),
            input,
        );
        debug_assert!(
            self.lines[r].chars().count() > c,
            "cursor {:?} exceeds max col {} at line {:?} after {:?}",
            self.cursor,
            self.lines[r].chars().count(),
            self.lines[r],
            input,
        );
    }

    pub fn insert_char(&mut self, c: char) {
        let (row, col) = self.cursor;
        let line = &mut self.lines[row];
        if let Some((i, _)) = line.char_indices().nth(col) {
            line.insert(i, c);
            self.cursor.1 += 1;
        }
    }

    pub fn insert_str(&mut self, s: &str) {
        let (row, col) = self.cursor;
        let line = &mut self.lines[row];
        debug_assert_eq!(
            line.char_indices().find(|(_, c)| *c == '\n'),
            None,
            "string given to insert_str must not contain newline",
        );
        if let Some((i, _)) = line.char_indices().nth(col) {
            line.insert_str(i, s);
            self.cursor.1 += s.chars().count();
        }
    }

    pub fn insert_tab(&mut self) {
        if !self.tab.is_empty() {
            let len = self.tab.len() - self.cursor.1 % self.tab.len();
            self.insert_str(&self.tab[..len]);
        }
    }

    pub fn insert_newline(&mut self) {
        let (row, col) = self.cursor;
        let line = &mut self.lines[row];
        let idx = line
            .char_indices()
            .nth(col)
            .map(|(i, _)| i)
            .unwrap_or(line.len() - 1);
        let next_line = line[idx..].to_string();
        line.truncate(idx);
        line.push(' ');
        self.lines.insert(row + 1, next_line);
        self.cursor = (row + 1, 0);
    }

    pub fn delete_char(&mut self) {
        let (row, col) = self.cursor;
        if col == 0 {
            if row > 0 {
                let line = self.lines.remove(row);
                let prev_line = &mut self.lines[row - 1];
                prev_line.pop(); // Remove trailing space
                prev_line.push_str(&line);
                self.cursor = (row - 1, prev_line.chars().count() - 1);
            }
            return;
        }

        let line = &mut self.lines[row];
        if let Some((i, _)) = line.char_indices().nth(col - 1) {
            line.remove(i);
            self.cursor.1 -= 1;
        }
    }

    pub fn cursor_forward(&mut self) {
        let (r, c) = self.cursor;
        if c + 1 >= self.lines[r].chars().count() {
            if r + 1 < self.lines.len() {
                self.cursor = (r + 1, 0);
            }
        } else {
            self.cursor.1 += 1;
        }
    }

    pub fn cursor_back(&mut self) {
        let (r, c) = self.cursor;
        if c == 0 {
            if r > 0 {
                self.cursor = (r - 1, self.lines[r - 1].chars().count() - 1);
            }
        } else {
            self.cursor.1 -= 1;
        }
    }

    pub fn cursor_down(&mut self) {
        let (r, c) = self.cursor;
        if r + 1 >= self.lines.len() {
            return;
        }
        self.cursor.0 += 1;
        let len = self.lines[r + 1].chars().count();
        if len <= c {
            self.cursor.1 = len - 1;
        }
    }

    pub fn cursor_up(&mut self) {
        let (r, c) = self.cursor;
        if r == 0 {
            return;
        }
        self.cursor.0 -= 1;
        let len = self.lines[r - 1].chars().count();
        if len <= c {
            self.cursor.1 = len - 1;
        }
    }

    pub fn cursor_start(&mut self) {
        self.cursor.1 = 0;
    }

    pub fn cursor_end(&mut self) {
        self.cursor.1 = self.lines[self.cursor.0].chars().count() - 1;
    }

    pub fn widget(&'a self) -> impl Widget + 'a {
        let mut lines = Vec::with_capacity(self.lines.len());
        for (i, l) in self.lines.iter().enumerate() {
            if i == self.cursor.0 {
                let (i, c) = l
                    .char_indices()
                    .nth(self.cursor.1)
                    .unwrap_or((l.len() - 1, ' '));
                let j = i + c.len_utf8();
                lines.push(Spans::from(vec![
                    Span::from(&l[..i]),
                    Span::styled(&l[i..j], Style::default().add_modifier(Modifier::REVERSED)),
                    Span::from(&l[j..]),
                ]));
            } else {
                lines.push(Spans::from(l.as_str()));
            }
        }
        let mut p = Paragraph::new(Text::from(lines)).style(self.style);
        if let Some(b) = &self.block {
            p = p.block(b.clone());
        }
        p
    }

    pub fn style(&mut self, style: Style) -> &mut Self {
        self.style = style;
        self
    }

    pub fn block(&mut self, block: Block<'a>) -> &mut Self {
        self.block = Some(block);
        self
    }

    pub fn remove_block(&mut self) -> &mut Self {
        self.block = None;
        self
    }

    pub fn tab(&mut self, tab: &'a str) -> &mut Self {
        assert!(
            tab.chars().all(|c| c == ' '),
            "tab string must consist of spaces but got {:?}",
            tab,
        );
        self.tab = tab;
        self
    }

    pub fn lines(&'a self) -> impl Iterator<Item = &'a str> {
        self.lines.iter().map(|l| &l[..l.len() - 1]) // Trim last whitespace
    }

    /// 0-base character-wise (row, col) cursor position.
    pub fn cursor(&self) -> (usize, usize) {
        self.cursor
    }
}
