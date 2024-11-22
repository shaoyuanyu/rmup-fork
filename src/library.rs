/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::{anyhow, Result};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::Accessor,
};
use rodio::{Decoder, Source};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::BufReader,
    path::{self, Path, PathBuf},
    time::Duration,
};

use crate::{playlist::Playlist, Load, Save};

pub mod album;
pub mod artist;
pub mod track;

use album::Album;
use artist::Artist;
use track::Track;

#[derive(Clone)]
pub struct Library {
    pub tracks: Playlist,
    known_paths: HashSet<PathBuf>,
}

pub fn get_track_data<P: AsRef<Path>>(path: P) -> Result<(Track, Artist, Album)> {
    let path = path.as_ref();
    if !path.is_file() {
        return Err(anyhow!("{} is not a file", path.display()));
    }

    let tagged_file = Probe::open(path)?.read()?;
    let mut length = tagged_file.properties().duration();
    if length == Duration::ZERO {
        let source = Decoder::new(BufReader::new(File::open(path)?))?;
        let sample_rate = f64::from(source.sample_rate());
        let channels = f64::from(source.channels());
        #[allow(clippy::cast_precision_loss)]
        let total_samples = source.count() as f64;
        length = Duration::from_secs_f64(total_samples / (sample_rate * channels));
    }
    let file_path = path
        .to_str()
        .expect("There is no good reason a path should not be convertable to a string")
        .to_string();

    let track = if let Some(tag) = tagged_file.primary_tag() {
        Track {
            title: tag.title().as_deref().map(std::borrow::ToOwned::to_owned),
            artist: tag.artist().as_deref().unwrap_or("Unknown").to_owned(),
            album: tag.album().as_deref().unwrap_or("Unknown").to_owned(),
            year: tag.year(),
            number: tag.track(),
            length,
            file_path,
        }
    } else {
        Track {
            title: None,
            artist: "Unknown".to_owned(),
            album: "Unknown".to_owned(),
            year: None,
            number: None,
            length,
            file_path,
        }
    };

    let mut artist = Artist::default().name(track.artist.as_str());

    let mut album = Album::default().name(track.album.as_str()).year(track.year);

    album.tracks.push(track.clone());

    artist.albums.push(Album::default().name("All Albums"));
    artist.albums[0].tracks.push(track.clone());
    artist.albums.push(album.clone());

    Ok((track, artist, album))
}

impl Library {
    pub fn new() -> Self {
        Self {
            tracks: Playlist::new("Library"),
            known_paths: HashSet::new(),
        }
    }

    pub fn add_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(anyhow!("{}: No such file or directory", path.display()));
        }
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                self.add_path(entry.path())?;
            }
        } else {
            if self.known_paths.contains::<PathBuf>(&path::absolute(path)?) {
                return Ok(());
            }

            self.known_paths.insert(path::absolute(path)?);
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().into_owned();
                match ext.as_str() {
                    "mp3" | "flac" | "aiff" | "m4a" | "ogg" | "opus" | "aac" | "wav" => {}
                    _ => {
                        return Ok(());
                    }
                }
            } else {
                return Ok(());
            }
            let (track, _, _) = get_track_data(path)?;

            // Add track to library
            self.tracks.tracks.push(track);
        }

        Ok(())
    }
}

impl Save for Library {
    fn save<P: AsRef<Path>>(&self, file_path: P) -> Result<()> {
        self.tracks.save(file_path)
    }
}

impl Load for Library {
    fn load<P: AsRef<Path>>(file_path: P) -> Result<Self>
    where
        Self: Sized,
    {
        let tracks = Playlist::load(file_path)?;
        let mut known_paths = HashSet::new();
        tracks.tracks.iter().for_each(|t| {
            known_paths.insert(PathBuf::from(&t.file_path));
        });
        Ok(Self {
            tracks,
            known_paths,
        })
    }
}
