/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::rc::Rc;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    Frame,
};
use tui_textarea::{CursorMove, TextArea};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Default)]
pub struct CommandLine<'a> {
    prompt: Rc<str>,
    pub textarea: TextArea<'a>,
}

impl<'a> CommandLine<'a> {
    pub fn render(
        &self,
        f: &mut Frame,
        command_line_chunk: Rect,
        show_cursor: bool,
        style: &Style,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                #[allow(clippy::cast_possible_truncation)]
                [
                    Constraint::Max(UnicodeWidthStr::width(self.prompt.as_ref()) as u16),
                    Constraint::Fill(1),
                ]
                .as_ref(),
            )
            .split(command_line_chunk);
        let mut textarea = self.textarea.clone();
        textarea.set_style(*style);
        textarea.set_cursor_line_style(*style);
        if !show_cursor {
            textarea.set_cursor_style(Style::default().add_modifier(Modifier::HIDDEN));
        }
        let prompt = Line::styled(self.prompt.as_ref(), style.add_modifier(Modifier::BOLD));

        f.render_widget(prompt, chunks[0]);
        f.render_widget(&textarea, chunks[1]);
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt = prompt.into();
    }

    pub fn clear_prompt(&mut self) {
        self.prompt = "".into();
    }

    pub fn reset(&mut self) {
        self.clear_prompt();
        self.clear_contents();
    }

    pub fn clear_contents(&mut self) {
        self.textarea.move_cursor(CursorMove::End);
        while self.textarea.delete_char() {}
    }

    pub fn get_contents(&self) -> String {
        self.textarea.lines()[0].clone()
    }
}
