/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use super::album::Album;

#[derive(Clone, Default, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Artist {
    /// Name of the artist
    pub name: String,

    /// All of the albums in the library by this artist. The very first item
    /// in the list should be a pseudo-album named "All Albums" whose list of
    /// tracks contains all of the tracks by this artist.
    pub albums: Vec<Album>,
}

impl Artist {
    pub fn name(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }

    pub fn get_album_index(&self, name: &str) -> Option<usize> {
        self.albums.iter().position(|a| a.name == name)
    }
}

/// Artists sort alphabetically
impl Ord for Artist {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.name == "All Artists" && other.name != "All Artists" {
            Ordering::Less
        } else if other.name == "All Artists" {
            Ordering::Greater
        } else {
            self.name.to_lowercase().cmp(&other.name.to_lowercase())
        }
    }
}

impl PartialOrd for Artist {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
