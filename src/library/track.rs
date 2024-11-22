/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::util::to_width;
use ratatui::widgets::ListItem;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Display, time::Duration};

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Track {
    /// Track name from metadata, if no name is present, filename will be
    /// displayed instead
    pub title: Option<String>,

    /// Artist name from metadata if present
    pub artist: String,

    /// Album name from metadata if present
    pub album: String,

    /// Year from metadata if present
    pub year: Option<u32>,

    /// Track number if present
    pub number: Option<u32>,

    /// Track duration
    pub length: Duration,

    /// Path to the audio file
    pub file_path: String,
}

/// Tracks sort first by artist. If they have the same artist, then they sort by
/// album. If they're on the same album, they then sort by track number. If
/// track number is not applicable to one or both of them, then they sort by
/// title. If title is not applicable to one or both of them, then the filename
/// is substituted for the title.
impl Ord for Track {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.artist != other.artist {
            self.artist.cmp(&other.artist)
        } else if self.album != other.album {
            self.album.cmp(&other.album)
        } else if let (Some(self_num), Some(other_num)) = (self.number, other.number) {
            self_num.cmp(&other_num)
        } else {
            let self_name = self
                .title
                .as_ref()
                .unwrap_or(&self.file_path)
                .to_lowercase();
            let other_name = other
                .title
                .as_ref()
                .unwrap_or(&other.file_path)
                .to_lowercase();
            self_name.cmp(&other_name)
        }
    }
}

impl PartialOrd for Track {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> From<&Track> for ListItem<'a> {
    fn from(val: &Track) -> Self {
        let title = val.title.as_ref().unwrap_or(&val.file_path);
        let artist = &val.artist;
        let album = &val.album;
        let year = val.year.map_or_else(String::new, |y| y.to_string());
        let length = format!(
            "{}:{:02}",
            val.length.as_secs() / 60,
            val.length.as_secs() % 60
        );

        let box_width = crossterm::terminal::size()
            .unwrap_or((80, 24))
            .0
            .saturating_sub(2) as usize;

        let col = box_width / 5;
        let col_widths = match box_width % 5 {
            0 => (col, col, col, col, col),
            1 => (col + 1, col, col, col, col),
            2 => (col + 1, col + 1, col, col, col),
            3 => (col + 1, col + 1, col + 1, col, col),
            4 => (col + 1, col + 1, col + 1, col + 1, col),
            _ => unreachable!("Any number mod 5 will be within 0..=4"),
        };

        ListItem::new(format!(
            "{}{}{}{}{}",
            to_width(title, col_widths.0, false),
            to_width(artist, col_widths.1, false),
            to_width(album, col_widths.2, false),
            to_width(&year, col_widths.3, true),
            to_width(&length, col_widths.4, true),
        ))
    }
}

impl Display for Track {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(title) = &self.title {
            f.write_str(title)
        } else {
            f.write_str(&self.file_path)
        }
    }
}
