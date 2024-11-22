/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::Result;
use async_std::sync::Mutex;
use crossterm::event::KeyEvent;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Text,
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    command::Command,
    config::{ConfOption, Config, UiColor},
    media_system::{MediaState, Queueable, Repeat},
    playlist::Playlist,
    Library, Mode,
};

mod command_line;
mod help_screen;
mod main_screen;
mod playlist_screen;

use command_line::CommandLine;
use help_screen::HelpScreen;
use main_screen::MainScreen;
use playlist_screen::PlaylistScreen;

#[derive(Clone, Copy)]
pub enum MovementDirection {
    Prev,
    Next,
    Top,
    Bottom,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
pub enum ScreenEnum {
    Main,
    Playlists,
    Help,
}

trait Screen {
    fn ui(&self, f: &mut Frame, page_chunk: Rect);
    fn style_panels(&mut self, selected: &Style, unselected: &Style);
    fn switch_panel(&mut self, direction: MovementDirection);
    fn switch_item(&mut self, direction: MovementDirection);
    fn update_lists(&mut self, normal_style: &Style);
    fn get_selected(&self, tracks_current_only: bool) -> Queueable;
}

pub struct UIList<'a, Item> {
    /// The items in the list
    list: Vec<Item>,

    /// The TUI List widget
    display: List<'a>,

    /// The List widget state
    state: ListState,
}

pub struct UI<'a> {
    main_screen: MainScreen<'a>,

    playlist_screen: PlaylistScreen<'a>,

    help_screen: HelpScreen<'a>,

    /// Playback progress bar
    playback_bar: Gauge<'a>,

    /// The current screen
    screen: ScreenEnum,

    /// Base widget style
    normal_style: Style,

    /// Highlight style for the currently selected panel
    highlight_selected: Style,

    /// Highlight style for the unselected panels
    highlight_unselected: Style,

    selected_playlist_index: Option<usize>,

    pub library: Library,

    pub command_line: CommandLine<'a>,
}

const NF_PLAY: char = '\u{f040a}';
const NF_PAUSE: char = '\u{f03e4}';
const NF_SHUFFLE: char = '\u{f049d}';
const NF_SHUFFLE_OFF: char = '\u{f049e}';
const NF_REPEAT: char = '\u{f0456}';
const NF_REPEAT_OFF: char = '\u{f0457}';
const NF_REPEAT_ONCE: char = '\u{f0458}';

impl<'a> UI<'a> {
    /// Create a new UI object, constructing the artist, album, and track lists
    /// from the given library.
    pub fn new(library: &'a Library, config: &'a Config, playlists: &[Playlist]) -> Self {
        use ScreenEnum::Main;

        let mut normal_style = Style::default();
        if let Some(bg_color) = config.colors.get(&UiColor::Bg) {
            normal_style = normal_style.bg(*bg_color);
        }
        if let Some(fg_color) = config.colors.get(&UiColor::Fg) {
            normal_style = normal_style.fg(*fg_color);
        }

        let mut highlight_selected = Style::default();
        if let Some(highlight_bg_color) = config.colors.get(&UiColor::HighlightBg) {
            highlight_selected = highlight_selected.bg(*highlight_bg_color);
        }
        if let Some(highlight_fg_color) = config.colors.get(&UiColor::HighlightFg) {
            highlight_selected = highlight_selected.fg(*highlight_fg_color);
        }

        let mut highlight_unselected = Style::default();
        //.bg(config.bg_color)
        //.fg(config.off_panel_highlight_color);
        if let Some(bg_color) = config.colors.get(&UiColor::Bg) {
            highlight_unselected = highlight_unselected.bg(*bg_color);
        }
        if let Some(off_panel_highlight_color) = config.colors.get(&UiColor::OffPanelHighlight) {
            highlight_unselected = highlight_unselected.fg(*off_panel_highlight_color);
        }

        let playback_bar = Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .gauge_style(normal_style)
            .ratio(0.0)
            .label("--:--/--:--");

        // Construct and configure UI
        let mut ui = Self {
            main_screen: MainScreen::new(library, &normal_style),
            playlist_screen: PlaylistScreen::new(playlists, &normal_style),
            help_screen: HelpScreen::new(config, &normal_style),
            playback_bar,
            screen: Main,
            normal_style,
            highlight_selected,
            highlight_unselected,
            selected_playlist_index: None,
            library: library.clone(),
            command_line: CommandLine::default(),
        };

        ui.style_panels();
        ui
    }

    pub fn update_library(&mut self, library: Library) {
        self.main_screen = MainScreen::new(&library, &self.normal_style);
        self.library = library;
    }

    /// Set the selection highlight for each panel based on which one is
    /// currently selected.
    fn style_panels(&mut self) {
        match self.screen {
            ScreenEnum::Main => self
                .main_screen
                .style_panels(&self.highlight_selected, &self.highlight_unselected),
            ScreenEnum::Playlists => self
                .playlist_screen
                .style_panels(&self.highlight_selected, &self.highlight_unselected),
            ScreenEnum::Help => self
                .help_screen
                .style_panels(&self.highlight_selected, &self.highlight_unselected),
        }
    }

    /// Build the UI and draw it to the terminal
    pub async fn draw<B: Backend>(
        &self,
        terminal: &mut Terminal<B>,
        media_state: &Arc<Mutex<MediaState>>,
        config: &Config,
        mode: &Mode,
    ) -> Result<()> {
        use ScreenEnum::{Help, Main, Playlists};

        let playback_bar = Self::build_playback_bar(self.playback_bar.clone(), media_state).await;
        let info_widget = Self::build_info_widget(self.normal_style, media_state, config).await;

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Min(3),
                        Constraint::Length(3),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                )
                .split(f.area());
            match &self.screen {
                Main => self.main_screen.ui(f, chunks[0]),
                Playlists => self.playlist_screen.ui(f, chunks[0]),
                Help => self.help_screen.ui(f, chunks[0]),
            }
            let playback_chunk = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(26), Constraint::Min(3)].as_ref())
                .split(chunks[1]);
            f.render_widget(info_widget, playback_chunk[0]);
            f.render_widget(playback_bar, playback_chunk[1]);
            let cursor = match mode {
                Mode::Normal => false,
                Mode::PlaylistEntry | Mode::CommandEntry => true,
            };
            self.command_line
                .render(f, chunks[2], cursor, &self.normal_style);
        })?;

        Ok(())
    }

    async fn build_info_widget(
        normal_style: Style,
        media_state: &Arc<Mutex<MediaState>>,
        config: &Config,
    ) -> Paragraph<'a> {
        let default_config = Config::default();
        let nerd_font_icons = *config
            .options
            .get(&ConfOption::NerdFontIcons)
            .unwrap_or_else(|| {
                default_config
                    .options
                    .get(&ConfOption::NerdFontIcons)
                    .expect("Default Config has NerdFontIcons option set")
            });
        let guard = media_state.lock().await;
        let playback_info = format!(
            " {} {} {} | {}",
            match guard.repeat {
                Repeat::On =>
                    if nerd_font_icons {
                        NF_REPEAT
                    } else {
                        'R'
                    },
                Repeat::Off =>
                    if nerd_font_icons {
                        NF_REPEAT_OFF
                    } else {
                        '-'
                    },
                Repeat::One =>
                    if nerd_font_icons {
                        NF_REPEAT_ONCE
                    } else {
                        '1'
                    },
            },
            if guard.shuffle {
                if nerd_font_icons {
                    NF_SHUFFLE
                } else {
                    'S'
                }
            } else if nerd_font_icons {
                NF_SHUFFLE_OFF
            } else {
                '-'
            },
            if guard.playing {
                if nerd_font_icons {
                    NF_PLAY
                } else {
                    '>'
                }
            } else if nerd_font_icons {
                NF_PAUSE
            } else {
                '-'
            },
            guard
                .current_track
                .as_ref()
                .map_or("Not Playing", |track| track
                    .title
                    .as_ref()
                    .map_or(&track.file_path, |title| title))
        );
        let info_text = Text::from(playback_info);
        let info_widget = Paragraph::new(info_text)
            .block(Block::default().borders(Borders::ALL))
            .style(normal_style);
        info_widget
    }

    async fn build_playback_bar(
        playback_bar: Gauge<'a>,
        media_state: &Arc<Mutex<MediaState>>,
    ) -> Gauge<'a> {
        let guard = media_state.lock().await;
        playback_bar
            .label(format!(
                "{}:{}/{}:{}",
                // Minutes into playback
                guard.current_track_progress.map_or_else(
                    || "--".to_owned(),
                    |duration| format!("{:02}", duration.as_secs() / 60)
                ),
                // Seconds into playback
                guard.current_track_progress.map_or_else(
                    || "--".to_owned(),
                    |duration| format!("{:02}", duration.as_secs() % 60)
                ),
                // Track length minutes
                guard.current_track.as_ref().map_or_else(
                    || "--".to_owned(),
                    |track| format!("{:02}", track.length.as_secs() / 60)
                ),
                // Track length seconds
                guard.current_track.as_ref().map_or_else(
                    || "--".to_owned(),
                    |track| format!("{:02}", track.length.as_secs() % 60)
                ),
            ))
            .ratio(
                if let (Some(progress), Some(track)) =
                    (guard.current_track_progress, &guard.current_track)
                {
                    let ratio = progress.as_secs_f64() / track.length.as_secs_f64();
                    if ratio < 0.0 || ratio.is_nan() {
                        0.0
                    } else if ratio > 1.0 {
                        1.0
                    } else {
                        ratio
                    }
                } else {
                    0.0
                },
            )
    }

    /// Move the selection up or down the list in the current panel, cycling
    /// back around after the beginning or end of the list.
    pub fn switch_item(&mut self, direction: MovementDirection) {
        match self.screen {
            ScreenEnum::Main => self.main_screen.switch_item(direction),
            ScreenEnum::Playlists => self.playlist_screen.switch_item(direction),
            ScreenEnum::Help => self.help_screen.switch_item(direction),
        }
    }

    /// Switch to the next panel.
    pub fn switch_panel(&mut self, direction: MovementDirection) {
        match self.screen {
            ScreenEnum::Main => self.main_screen.switch_panel(direction),
            ScreenEnum::Playlists => self.playlist_screen.switch_panel(direction),
            ScreenEnum::Help => self.help_screen.switch_panel(direction),
        }
        self.style_panels();
    }

    pub fn switch_screen(&mut self, screen: ScreenEnum) {
        if self.screen != screen {
            self.screen = screen;
            self.update_lists();
        }
    }

    /// Change which album and track lists will be shown in the UI based on
    /// which artist and album list items are selected.
    pub fn update_lists(&mut self) {
        match self.screen {
            ScreenEnum::Main => self.main_screen.update_lists(&self.normal_style),
            ScreenEnum::Playlists => self.playlist_screen.update_lists(&self.normal_style),
            ScreenEnum::Help => self.help_screen.update_lists(&self.normal_style),
        }

        // Ensure panels are styled correctly after replacing them
        self.style_panels();
    }

    /// If artist is selected, return the artist track list. If album is
    /// selected, return the album track list. If track is selected, return a
    /// Vec containing just that track if `tracks_current_only == true`, or
    /// otherwise a Vec containing all of the tracks in the list starting from
    /// the current track and wrapping around at the end of the list.
    pub fn get_selected(&self, tracks_current_only: bool) -> Queueable {
        match self.screen {
            ScreenEnum::Main => self.main_screen.get_selected(tracks_current_only),
            ScreenEnum::Playlists => self.playlist_screen.get_selected(tracks_current_only),
            ScreenEnum::Help => self.help_screen.get_selected(tracks_current_only),
        }
    }

    /// Return the command that corresponds to the given input.
    pub fn get_key_command(&self, ke: KeyEvent, config: &Config) -> Command {
        config
            .keybinds
            .get(&ke.code)
            .map_or(Command::Nop, |command| {
                let command = command.clone();
                match (self.screen, &self.playlist_screen.panel, &command) {
                    (
                        ScreenEnum::Playlists,
                        playlist_screen::Panel::Playlists,
                        Command::SelectPlaylist,
                    ) => command,
                    (
                        ScreenEnum::Playlists,
                        playlist_screen::Panel::Playlists,
                        Command::PlaylistAdd,
                    )
                    | (_, _, Command::SelectPlaylist) => Command::Nop,
                    (_, _, _) => command,
                }
            })
    }

    pub fn add_playlist(&mut self, playlist: &Playlist) {
        self.playlist_screen
            .playlist_list
            .list
            .push(playlist.clone());
        let listitems: Vec<ListItem> = self
            .playlist_screen
            .playlist_list
            .list
            .iter()
            .map(|pl| ListItem::new(pl.name.clone()))
            .collect();
        let list_display = List::new(listitems)
            .block(Block::default().title("Playlists").borders(Borders::ALL))
            .style(self.normal_style);
        self.playlist_screen.playlist_list.display = list_display;
    }

    pub fn add_selected_to_playlist(&mut self) {
        let message = if let Some(index) = self.selected_playlist_index {
            let mut tracks = self.get_selected(true).get_tracks();
            if let Some(playlist) = self.playlist_screen.playlist_list.list.get_mut(index) {
                let msg = if tracks.len() == 1 {
                    let track = tracks.first().expect("Length == 1");
                    format!("Added \"{}\" to {}", track, playlist.name)
                } else {
                    format!("Added {} tracks to {}", tracks.len(), playlist.name)
                };
                playlist.add(&mut tracks);
                msg
            } else {
                "Error: selected playlist index does not exist".into()
            }
        } else {
            "Select a playlist to add tracks".into()
        };

        self.command_line.reset();
        self.command_line.textarea.insert_str(&message);
    }

    /// Set the currently highlighted item in the playlist list to the selected
    /// playlist.
    pub fn select_current_playlist(&mut self) {
        if let Some(new_index) = self.playlist_screen.playlist_list.state.selected() {
            let listitems: Vec<ListItem> = self
                .playlist_screen
                .playlist_list
                .list
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    if i == new_index {
                        ListItem::new(format!("*{}", p.name))
                    } else {
                        ListItem::new(p.name.clone())
                    }
                })
                .collect();
            let list_display = List::new(listitems)
                .block(Block::default().title("Playlists").borders(Borders::ALL))
                .style(self.normal_style);
            self.playlist_screen.playlist_list.display = list_display;
            self.selected_playlist_index = Some(new_index);
        }
    }

    pub fn selected_playlist(&self) -> Option<&Playlist> {
        self.selected_playlist_index
            .map(|index| &self.playlist_screen.playlist_list.list[index])
    }
}
