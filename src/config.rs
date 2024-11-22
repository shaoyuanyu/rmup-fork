/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{collections::HashMap, fs::File, path::Path};

use anyhow::Result;
use crossterm::event::KeyCode;
use map_macro::hash_map;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::{command::Command, ui::ScreenEnum, Load, Save};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub colors: HashMap<UiColor, Color>,
    pub keybinds: HashMap<KeyCode, Command>,
    pub options: HashMap<ConfOption, bool>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UiColor {
    Fg,
    HighlightFg,
    Bg,
    HighlightBg,
    OffPanelHighlight,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConfOption {
    NerdFontIcons,
    GaplessPlayback,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            colors: hash_map! {
                UiColor::OffPanelHighlight => Color::Red,
                UiColor::HighlightFg => Color::Black,
                UiColor::HighlightBg => Color::White,
            },
            keybinds: hash_map! {
                KeyCode::Char('k') => Command::Up,
                KeyCode::Up => Command::Up,
                KeyCode::Char('j') => Command::Down,
                KeyCode::Down => Command::Down,
                KeyCode::Char(' ') => Command::TogglePlay,
                KeyCode::Char(',') => Command::PrevTrack,
                KeyCode::Char('.') => Command::NextTrack,
                KeyCode::Enter => Command::QueueAndPlay,
                KeyCode::Char('r') => Command::ToggleRepeat,
                KeyCode::Char('s') => Command::ToggleShuffle,
                KeyCode::Char('g') => Command::GotoTop,
                KeyCode::Char('G') => Command::GotoBottom,
                KeyCode::Tab => Command::NextPanel,
                KeyCode::BackTab => Command::PrevPanel,
                KeyCode::Char('1') => Command::GotoScreen(ScreenEnum::Main),
                KeyCode::Char('2') => Command::GotoScreen(ScreenEnum::Playlists),
                KeyCode::Char('0') => Command::GotoScreen(ScreenEnum::Help),
                KeyCode::F(1) => Command::GotoScreen(ScreenEnum::Help),
                KeyCode::Char('n') => Command::NewPlaylist(None),
                KeyCode::Char('p') => Command::PlaylistAdd,
                KeyCode::Char('x') => Command::SelectPlaylist,
                KeyCode::Char('q') => Command::Quit,
                KeyCode::Char(':') => Command::EnterCommand,
            },
            options: hash_map! {
                ConfOption::NerdFontIcons => true,
                ConfOption::GaplessPlayback => true,
            },
        }
    }
}

impl Save for Config {
    fn save<P: AsRef<Path>>(&self, file_path: P) -> Result<()> {
        let config_file = File::create(file_path)?;
        Ok(serde_yml::to_writer(config_file, self)?)
    }
}

impl Load for Config {
    fn load<P: AsRef<Path>>(file_path: P) -> Result<Self>
    where
        Self: Sized,
    {
        let config_file = File::open(file_path)?;
        Ok(serde_yml::from_reader(config_file)?)
    }
}

impl Config {
    pub fn get_command_keys(&self, command: &Command) -> Vec<KeyCode> {
        self.keybinds
            .clone()
            .into_iter()
            .filter_map(|(k, v)| if v == *command { Some(k) } else { None })
            .collect::<Vec<_>>()
    }
}
