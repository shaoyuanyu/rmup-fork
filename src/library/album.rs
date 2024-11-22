/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use ratatui::widgets::ListItem;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use super::track::Track;
use crate::util::to_width;

#[derive(Clone, Default, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Album {
    /// Name of the album
    pub name: String,

    /// Year from metadata, if present in tracks
    pub year: Option<u32>,

    /// Tracks in the album.
    pub tracks: Vec<Track>,
}

impl Album {
    pub fn name(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }

    pub const fn year(mut self, year: Option<u32>) -> Self {
        self.year = year;
        self
    }
}

/// Albums sort alphabetically
impl Ord for Album {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.name == "All Albums" && other.name != "All Albums" {
            Ordering::Less
        } else if other.name == "All Albums" {
            Ordering::Greater
        } else {
            self.name.to_lowercase().cmp(&other.name.to_lowercase())
        }
    }
}

impl PartialOrd for Album {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> From<&Album> for ListItem<'a> {
    fn from(val: &Album) -> Self {
        let title = val.name.clone();
        let year = val.year.map_or_else(String::new, |y| y.to_string());
        let term_width = crossterm::terminal::size().unwrap_or((80, 24)).0 as usize;
        // The albums pane takes up half of the terminal width
        let block_width = term_width / 2;
        // The 2 sides of the block take up 1 char each
        let text_width = block_width.saturating_sub(2);
        // The year takes up 4 chars. What remains is for the album title
        let title_width = text_width.saturating_sub(4);
        ListItem::new(format!("{}{}", to_width(&title, title_width, false), year))
    }
}
