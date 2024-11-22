/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::mem;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::{
    library::{album::Album, artist::Artist, track::Track},
    media_system::Queueable,
    Library,
};

use super::{MovementDirection, Screen, UIList};

enum Panel {
    Artists,
    Albums,
    Tracks,
}

pub struct MainScreen<'a> {
    /// The list of artists that will display in the UI
    pub artist_list: UIList<'a, Artist>,

    /// The list of albums that will display in the UI
    pub album_list: UIList<'a, Album>,

    /// The list of tracks that will display in the UI
    pub track_list: UIList<'a, Track>,

    panel: Panel,
}

impl<'a> MainScreen<'a> {
    pub fn new(library: &Library, normal_style: &Style) -> Self {
        let (artist_list, album_list) = library.tracks.get_artists_albums();

        // Create artist list from library
        let artist_listitems: Vec<ListItem> = artist_list
            .iter()
            .map(|e| ListItem::new(e.name.clone()))
            .collect();
        let artist_list_display = List::new(artist_listitems)
            .block(Block::default().title("Artist").borders(Borders::ALL))
            .style(*normal_style);
        let artist_list_state = ListState::default();

        let mut artist_list = UIList {
            list: artist_list,
            display: artist_list_display,
            state: artist_list_state,
        };

        // Create album list from library
        let album_listitems: Vec<ListItem> =
            album_list.iter().map(std::convert::Into::into).collect();
        let album_list_display = List::new(album_listitems)
            .block(Block::default().title("Album").borders(Borders::ALL))
            .style(*normal_style);
        let album_list_state = ListState::default();

        let mut album_list = UIList {
            list: album_list,
            display: album_list_display,
            state: album_list_state,
        };

        // Create track list from library
        let track_list = library.tracks.tracks.clone();
        let track_listitems: Vec<ListItem> =
            track_list.iter().map(std::convert::Into::into).collect();
        let track_list_display = List::new(track_listitems)
            .block(Block::default().title("Track").borders(Borders::ALL))
            .style(*normal_style);
        let track_list_state = ListState::default();

        let mut track_list = UIList {
            list: track_list.clone(),
            display: track_list_display.clone(),
            state: track_list_state,
        };

        artist_list.state.select(Some(0));
        album_list.state.select(Some(0));
        track_list.state.select(Some(0));

        Self {
            artist_list,
            album_list,
            track_list,
            panel: Panel::Artists,
        }
    }
}

impl<'a> Screen for MainScreen<'a> {
    fn ui(&self, f: &mut ratatui::Frame, page_chunk: Rect) {
        use ratatui::layout::Direction;

        // Split the screen into top and bottom halves
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Min(3)].as_ref())
            .split(page_chunk);

        // Split the top half into left and right halves
        let upper_chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);

        // Render artist list in top left
        let mut artist_list_state = self.artist_list.state.clone();
        f.render_stateful_widget(
            self.artist_list.display.clone(),
            upper_chunk[0],
            &mut artist_list_state,
        );
        // Render album list in top right
        let mut album_list_state = self.album_list.state.clone();
        f.render_stateful_widget(
            self.album_list.display.clone(),
            upper_chunk[1],
            &mut album_list_state,
        );
        // Render track list in bottom
        let mut track_list_state = self.track_list.state.clone();
        f.render_stateful_widget(
            self.track_list.display.clone(),
            chunks[1],
            &mut track_list_state,
        );
    }

    fn style_panels(&mut self, selected: &Style, unselected: &Style) {
        match &self.panel {
            Panel::Artists => {
                self.artist_list.display =
                    self.artist_list.display.clone().highlight_style(*selected);
                self.album_list.display =
                    self.album_list.display.clone().highlight_style(*unselected);
                self.track_list.display =
                    self.track_list.display.clone().highlight_style(*unselected);
            }
            Panel::Albums => {
                self.artist_list.display = self
                    .artist_list
                    .display
                    .clone()
                    .highlight_style(*unselected);
                self.album_list.display =
                    self.album_list.display.clone().highlight_style(*selected);
                self.track_list.display =
                    self.track_list.display.clone().highlight_style(*unselected);
            }
            Panel::Tracks => {
                self.artist_list.display = self
                    .artist_list
                    .display
                    .clone()
                    .highlight_style(*unselected);
                self.album_list.display =
                    self.album_list.display.clone().highlight_style(*unselected);
                self.track_list.display =
                    self.track_list.display.clone().highlight_style(*selected);
            }
        }
    }

    fn switch_panel(&mut self, direction: MovementDirection) {
        use MovementDirection::{Next, Prev};
        use Panel::{Albums, Artists, Tracks};

        match direction {
            Next => {
                self.panel = match self.panel {
                    Artists => Albums,
                    Albums => Tracks,
                    Tracks => Artists,
                }
            }
            Prev => {
                self.panel = match self.panel {
                    Artists => Tracks,
                    Albums => Artists,
                    Tracks => Albums,
                }
            }
            _ => {}
        }
    }

    fn switch_item(&mut self, direction: MovementDirection) {
        use MovementDirection::{Bottom, Next, Prev, Top};
        use Panel::{Albums, Artists, Tracks};

        let current_list_len = match self.panel {
            Artists => self.artist_list.list.len(),
            Albums => self.album_list.list.len(),
            Tracks => self.track_list.list.len(),
        };

        if current_list_len == 0 {
            return;
        }

        let current_list_state = match self.panel {
            Artists => &mut self.artist_list.state,
            Albums => &mut self.album_list.state,
            Tracks => &mut self.track_list.state,
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
        // Get the albums list of the currently selected artist
        let artist_selected_index = self.artist_list.state.selected().unwrap_or_default();
        let list = self.artist_list.list[artist_selected_index].albums.clone();

        // Convert that albums list into a Vec of ListItems to create a List widget
        let listitems: Vec<ListItem> = list.iter().map(std::convert::Into::into).collect();
        let list_display = List::new(listitems)
            .block(Block::default().title("Album").borders(Borders::ALL))
            .style(*normal_style);
        // Overwrite the album list in the UI, keeping the same ListState to preserve selected index
        self.album_list = UIList {
            list,
            display: list_display,
            state: mem::take(&mut self.album_list.state),
        };

        // If selected index is past the end of the list, put it at the end of the list
        if self.album_list.state.selected().unwrap_or_default() >= self.album_list.list.len() {
            self.album_list
                .state
                .select(Some(self.album_list.list.len() - 1));
        }

        // Get the track list of the currently selected album

        let list = {
            let album_selected_index = self.album_list.state.selected().unwrap_or_default();
            self.album_list.list[album_selected_index].tracks.clone()
        };

        // Convert that track list into a Vec of ListItems to create a List widget
        let listitems: Vec<ListItem> = list.iter().map(std::convert::Into::into).collect();
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
        use Panel::{Albums, Artists, Tracks};

        match self.panel {
            Artists => {
                let artist_index = self.artist_list.state.selected().unwrap_or_default();
                Queueable::Artist(self.artist_list.list[artist_index].clone())
            }
            Albums => {
                let album_index = self.album_list.state.selected().unwrap_or_default();
                Queueable::Album(self.album_list.list[album_index].clone())
            }
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
        }
    }
}
