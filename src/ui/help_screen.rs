/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crossterm::event::KeyCode;
use ratatui::{
    style::Style,
    text::Text,
    widgets::{Block, Borders, Paragraph},
};

use crate::{command::Command, config::Config, media_system::Queueable};

use super::{Screen, ScreenEnum};

pub struct HelpScreen<'a> {
    help_page: Paragraph<'a>,
}

impl<'a> HelpScreen<'a> {
    pub fn new(config: &'a Config, normal_style: &Style) -> Self {
        let help_text = Text::from(format!(
            "Up:                {}\n\
             Down:              {}\n\
             Play/Pause:        {}\n\
             Previous track:    {}\n\
             Next track:        {}\n\
             Enqueue:           {}\n\
             Repeat:            {}\n\
             Shuffle:           {}\n\
             Goto top:          {}\n\
             Goto bottom:       {}\n\
             Next panel:        {}\n\
             Previous panel:    {}\n\
             Main screen:       {}\n\
             Playlist screen:   {}\n\
             Help screen:       {}\n\
             New playlist:      {} (Playlist screen only)\n\
             Select playlist:   {} (Playlist screen only)\n\
             Add to playlist:   {}\n\
             Quit:              {}",
            display_keys(&config.get_command_keys(&Command::Up)),
            display_keys(&config.get_command_keys(&Command::Down)),
            display_keys(&config.get_command_keys(&Command::TogglePlay)),
            display_keys(&config.get_command_keys(&Command::PrevTrack)),
            display_keys(&config.get_command_keys(&Command::NextTrack)),
            display_keys(&config.get_command_keys(&Command::QueueAndPlay)),
            display_keys(&config.get_command_keys(&Command::ToggleRepeat)),
            display_keys(&config.get_command_keys(&Command::ToggleShuffle)),
            display_keys(&config.get_command_keys(&Command::GotoTop)),
            display_keys(&config.get_command_keys(&Command::GotoBottom)),
            display_keys(&config.get_command_keys(&Command::NextPanel)),
            display_keys(&config.get_command_keys(&Command::PrevPanel)),
            display_keys(&config.get_command_keys(&Command::GotoScreen(ScreenEnum::Main))),
            display_keys(&config.get_command_keys(&Command::GotoScreen(ScreenEnum::Playlists))),
            display_keys(&config.get_command_keys(&Command::GotoScreen(ScreenEnum::Help))),
            display_keys(&config.get_command_keys(&Command::NewPlaylist(None))),
            display_keys(&config.get_command_keys(&Command::SelectPlaylist)),
            display_keys(&config.get_command_keys(&Command::PlaylistAdd)),
            display_keys(&config.get_command_keys(&Command::Quit)),
        ));
        let help_page = Paragraph::new(help_text)
            .block(Block::default().title("Help").borders(Borders::ALL))
            .style(*normal_style);

        Self { help_page }
    }
}

impl<'a> Screen for HelpScreen<'a> {
    fn ui(&self, f: &mut ratatui::Frame, page_chunk: ratatui::layout::Rect) {
        f.render_widget(self.help_page.clone(), page_chunk);
    }

    fn style_panels(
        &mut self,
        _selected: &ratatui::style::Style,
        _unselected: &ratatui::style::Style,
    ) {
    }

    fn switch_panel(&mut self, _direction: super::MovementDirection) {}

    fn switch_item(&mut self, _direction: super::MovementDirection) {}

    fn update_lists(&mut self, _normal_style: &ratatui::style::Style) {}

    fn get_selected(&self, _tracks_current_only: bool) -> Queueable {
        Queueable::Empty
    }
}

fn display_keys(keys: &[KeyCode]) -> String {
    let mut s = String::new();
    for (i, k) in keys.iter().enumerate() {
        let key_string = match k {
            KeyCode::Char(' ') => "Space".to_owned(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Backspace => "Backspace".to_owned(),
            KeyCode::Enter => "Enter".to_owned(),
            KeyCode::Left => "Left".to_owned(),
            KeyCode::Right => "Right".to_owned(),
            KeyCode::Up => "Up".to_owned(),
            KeyCode::Down => "Down".to_owned(),
            KeyCode::Home => "Home".to_owned(),
            KeyCode::End => "End".to_owned(),
            KeyCode::PageUp => "Page Up".to_owned(),
            KeyCode::PageDown => "Page Down".to_owned(),
            KeyCode::Tab => "Tab".to_owned(),
            KeyCode::BackTab => "Shift+Tab".to_owned(),
            KeyCode::Delete => "Delete".to_owned(),
            KeyCode::Insert => "Insert".to_owned(),
            KeyCode::F(n) => format!("F{n}"),
            KeyCode::Null => "Null".to_owned(),
            KeyCode::Esc => "Esc".to_owned(),
            KeyCode::CapsLock => "Caps Lock".to_owned(),
            KeyCode::ScrollLock => "Scroll Lock".to_owned(),
            KeyCode::NumLock => "Num Lock".to_owned(),
            KeyCode::PrintScreen => "Print Screen".to_owned(),
            KeyCode::Pause => "Pause".to_owned(),
            KeyCode::Menu => "Menu".to_owned(),
            _ => "Invalid Key Code".to_owned(),
        };
        if i == 0 {
            s.push_str(&key_string);
        } else {
            s.push_str(&format!(", {key_string}"));
        }
    }
    s
}
