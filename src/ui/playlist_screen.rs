/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::mem;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::{library::track::Track, media_system::Queueable, playlist::Playlist};

use super::{MovementDirection, Screen, UIList};

#[derive(PartialEq, Eq)]
pub enum Panel {
    Playlists,
    Tracks,
}

pub struct PlaylistScreen<'a> {
    /// The list of tracks that will display in the UI
    pub track_list: UIList<'a, Track>,

    pub playlist_list: UIList<'a, Playlist>,

    pub panel: Panel,
}

impl<'a> PlaylistScreen<'a> {
    pub fn new(playlists: &[Playlist], normal_style: &Style) -> Self {
        let playlist_listitems: Vec<ListItem> = playlists
            .iter()
            .map(|pl| ListItem::new(pl.name.clone()))
            .collect();
        let mut playlist_list = UIList {
            list: playlists.to_owned(),
            display: List::new(playlist_listitems)
                .block(Block::default().title("Playlist").borders(Borders::ALL))
                .style(*normal_style),
            state: ListState::default(),
        };

        let tracks: Vec<Track> = playlists
            .first()
            .map_or_else(Vec::new, |pl| pl.tracks.clone());
        let track_listitems: Vec<ListItem> = tracks
            .iter()
            .map(|t| {
                t.title.as_ref().map_or_else(
                    || ListItem::new(t.file_path.clone()),
                    |title| ListItem::new(title.clone()),
                )
            })
            .collect();
        let mut track_list = UIList {
            list: tracks,
            display: List::new(track_listitems)
                .block(Block::default().title("Track").borders(Borders::ALL))
                .style(*normal_style),
            state: ListState::default(),
        };

        playlist_list.state.select(Some(0));
        track_list.state.select(Some(0));

        Self {
            track_list,
            playlist_list,
            panel: Panel::Playlists,
        }
    }
}

impl<'a> Screen for PlaylistScreen<'a> {
    fn ui(&self, f: &mut Frame, page_chunk: Rect) {
        // Split the screen into left and right halves
        let upper_chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(page_chunk);

        // Render artist list in top left
        let mut playlist_list_state = self.playlist_list.state.clone();
        f.render_stateful_widget(
            self.playlist_list.display.clone(),
            upper_chunk[0],
            &mut playlist_list_state,
        );

        // Render track list in bottom
        let mut track_list_state = self.track_list.state.clone();
        f.render_stateful_widget(
            self.track_list.display.clone(),
            upper_chunk[1],
            &mut track_list_state,
        );
    }

    fn style_panels(&mut self, selected: &Style, unselected: &Style) {
        use Panel::{Playlists, Tracks};

        match &self.panel {
            Tracks => {
                self.playlist_list.display = self
                    .playlist_list
                    .display
                    .clone()
                    .highlight_style(*unselected);

                self.track_list.display =
                    self.track_list.display.clone().highlight_style(*selected);
            }
            Playlists => {
                self.playlist_list.display = self
                    .playlist_list
                    .display
                    .clone()
                    .highlight_style(*selected);
                self.track_list.display =
                    self.track_list.display.clone().highlight_style(*unselected);
            }
        }
    }

    fn switch_panel(&mut self, direction: MovementDirection) {
        use MovementDirection::{Next, Prev};
        use Panel::{Playlists, Tracks};

        match direction {
            Prev | Next => {
                self.panel = match self.panel {
                    Tracks => Playlists,
                    Playlists => Tracks,
                }
            }
            _ => {}
        }
    }

    fn switch_item(&mut self, direction: MovementDirection) {
        use MovementDirection::{Bottom, Next, Prev, Top};
        use Panel::{Playlists, Tracks};

        let current_list_len = match self.panel {
            Tracks => self.track_list.list.len(),
            Playlists => self.playlist_list.list.len(),
        };

        if current_list_len == 0 {
            return;
        }

        let current_list_state = match self.panel {
            Tracks => &mut self.track_list.state,
            Playlists => &mut self.playlist_list.state,
        };

        let mut selected = current_list_state.selected().unwrap_or_default();

        match direction {
            Prev => {
                if selected == 0 {
                    selected = current_list_len - 1;
                } else {
                    selected -= 1;
                }
            }
            Next => {
                if selected == current_list_len - 1 {
                    selected = 0;
                } else {
                    selected += 1;
                }
            }
            Top => selected = 0,
            Bottom => selected = current_list_len - 1,
        }
        current_list_state.select(Some(selected));
    }

    fn update_lists(&mut self, normal_style: &Style) {
        let list = {
            let playlist_selected_index = self.playlist_list.state.selected().unwrap_or_default();
            self.playlist_list
                .list
                .get(playlist_selected_index)
                .map_or_else(Vec::new, |playlist| playlist.tracks.clone())
        };

        // Convert that track list into a Vec of ListItems to create a List widget
        let listitems: Vec<ListItem> = list
            .iter()
            .map(|track| {
                let title = track
                    .title
                    .clone()
                    .unwrap_or_else(|| track.file_path.clone());
                ListItem::new(title)
            })
            .collect();
        let list_display = List::new(listitems)
            .block(Block::default().title("Track").borders(Borders::ALL))
            .style(*normal_style);
        // Overwrite the track list in the UI, keeping the same ListState to preserve selected index
        self.track_list = UIList {
            list,
            display: list_display,
            state: mem::take(&mut self.track_list.state),
        };

        // If selected index is past the end of the list, put it at the end of the list
        if self.track_list.state.selected().unwrap_or_default() >= self.track_list.list.len()
            && !self.track_list.list.is_empty()
        {
            self.track_list
                .state
                .select(Some(self.track_list.list.len() - 1));
        } else if self.track_list.list.is_empty() {
            self.track_list.state.select(None);
        }
    }

    fn get_selected(&self, tracks_current_only: bool) -> Queueable {
        use Panel::{Playlists, Tracks};

        match self.panel {
            Tracks => {
                let track_index = self.track_list.state.selected().unwrap_or_default();

                if tracks_current_only {
                    Queueable::TrackList(vec![self.track_list.list[track_index].clone()].into())
                } else {
                    let mut v = self.track_list.list[track_index..].to_vec();
                    v.append(&mut self.track_list.list[..track_index].to_vec());
                    Queueable::TrackList(v.into())
                }
            }
            Playlists => {
                let playlist_index = self.playlist_list.state.selected().unwrap_or_default();
                self.playlist_list.list.get(playlist_index).map_or_else(
                    || Queueable::Empty,
                    |playlist| Queueable::Playlist(playlist.clone()),
                )
            }
        }
    }
}
