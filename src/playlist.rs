/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Lines, Write},
    iter::Enumerate,
    mem,
    path::Path,
    sync::LazyLock,
    time::Duration,
};

use anyhow::{anyhow, Result};
use regex::Regex;

use crate::{
    library::{album::Album, artist::Artist, track::Track},
    traits::{Load, Save},
};

#[derive(Debug, Clone)]
pub struct Playlist {
    pub name: String,
    pub tracks: Vec<Track>,
}

impl Playlist {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            tracks: Vec::new(),
        }
    }

    pub fn add(&mut self, tracks: &mut Vec<Track>) {
        self.tracks.append(tracks);
    }

    /// Get the list of artists and albums of all the tracks in the playlist
    pub fn get_artists_albums(&self) -> (Vec<Artist>, Vec<Album>) {
        let mut artists: HashMap<String, Artist> = HashMap::new();
        let mut albums: HashMap<String, Album> = HashMap::new();

        // A pseudo-album which contains all of the tracks from all albums in
        // the playlist
        let mut all_albums = Album {
            name: "All Albums".to_owned(),
            year: None,
            tracks: Vec::new(),
        };

        // Construct all of the albums from the track list
        for track in &self.tracks {
            all_albums.tracks.push(track.clone());

            if let Some(album) = albums.get_mut(&track.album) {
                album.tracks.push(track.clone());
            } else {
                let album = Album {
                    name: track.album.clone(),
                    year: track.year,
                    tracks: vec![track.clone()],
                };
                albums.insert(track.album.clone(), album);
            }
        }

        // Construct all of the artists from the albums and sort the tracks in
        // each album
        for album in albums.values_mut() {
            album.tracks.sort();
            for track in &album.tracks {
                if let Some(artist) = artists.get_mut(&track.artist) {
                    let all_index = artist
                        .get_album_index("All Albums")
                        .expect("'All Albums' pseudo-album should always exist");
                    artist.albums[all_index].tracks.push(track.clone());
                    if artist.get_album_index(&album.name).is_none() {
                        artist.albums.push(album.clone());
                    }
                } else {
                    let artist_all_albums = Album {
                        name: "All Albums".to_owned(),
                        year: None,
                        tracks: vec![track.clone()],
                    };
                    let artist = Artist {
                        name: track.artist.clone(),
                        albums: vec![artist_all_albums, album.clone()],
                    };
                    artists.insert(artist.name.clone(), artist);
                }
            }
        }

        // Sort the albums in each artist
        for artist in artists.values_mut() {
            artist.albums.sort();
        }

        albums.insert("All Albums".to_owned(), all_albums);
        let mut albums: Vec<Album> = albums.values_mut().map(mem::take).collect();
        albums.sort();

        // A pseudo-artist which contains all of the albums from all artists in
        // the playlist
        let all_artists = Artist {
            name: "All Artists".to_owned(),
            albums: albums.clone(),
        };

        artists.insert("All Artists".to_owned(), all_artists);
        let mut artists: Vec<Artist> = artists.values_mut().map(mem::take).collect();
        artists.sort();

        (artists, albums)
    }
}

impl Save for Playlist {
    /// Save playlist to an m3u8 file
    fn save<P: AsRef<Path>>(&self, file_path: P) -> Result<()> {
        let mut file = File::create(file_path)?;
        writeln!(file, "#EXTM3U")?;
        writeln!(file, "#PLAYLIST:{}", self.name)?;
        for track in &self.tracks {
            writeln!(file, "#EXTART:{}", &track.artist)?;
            writeln!(file, "#EXTALB:{}", &track.album)?;

            let mut extinf = format!("#EXTINF:{}", track.length.as_secs());
            if let Some(year) = track.year {
                extinf.push_str(format!(" year={year}").as_str());
            }
            if let Some(number) = track.number {
                extinf.push_str(format!(" number={number}").as_str());
            }
            extinf.push(',');
            if let Some(title) = &track.title {
                extinf.push_str(title);
            }
            writeln!(file, "{extinf}")?;

            writeln!(file, "{}", track.file_path)?;
        }
        Ok(())
    }
}

static COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#.*$").expect("Known valid regex"));
static PLAYLIST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#PLAYLIST:.*$").expect("Known valid regex"));
static INF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#EXTINF:[0-9]*(\s.+)*,.*$").expect("Known valid regex"));
static ALB_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#EXTALB:.*$").expect("Known valid regex"));
static ART_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#EXTART:.*$").expect("Known valid regex"));

fn check_header<P: AsRef<Path>>(header: &str, file_path: P) -> Result<()> {
    if header != "#EXTM3U" {
        return Err(anyhow!(
            "{}: Invalid m3u8 header: '{header}'",
            file_path.as_ref().display()
        ));
    }
    Ok(())
}

fn parse_lines<P: AsRef<Path>>(
    lines: &mut Enumerate<Lines<BufReader<File>>>,
    file_path: P,
) -> Result<(String, Vec<Track>)> {
    let mut name = String::new();
    let mut tracks = Vec::new();
    let mut track_artist = None;
    let mut track_album = None;
    let mut track_duration = None;
    let mut track_name = None;
    let mut track_year = None;
    let mut track_number = None;

    for (linenum, line) in lines {
        let line = line?;
        if PLAYLIST_RE.is_match(&line) {
            line.split_once(':')
                .ok_or_else(|| {
                    anyhow!(
                        "Error parsing playlist '{}' line {linenum}: `#PLAYLIST:` cannot be empty",
                        file_path.as_ref().display()
                    )
                })?
                .1
                .clone_into(&mut name);
        } else if ART_RE.is_match(&line) {
            track_artist = Some(
                line.split_once(':')
                    .ok_or_else(|| {
                        anyhow!(
                            "Error parsing playlist '{}' line {linenum}: `#EXTART:` cannot be empty",
                            file_path.as_ref().display()
                        )
                    })?
                    .1
                    .to_owned(),
            );
        } else if ALB_RE.is_match(&line) {
            track_album = Some(
                line.split_once(':')
                    .ok_or_else(|| {
                        anyhow!(
                            "Error parsing playlist '{}' line {linenum}: `#EXTALB:` cannot be empty",
                            file_path.as_ref().display()
                        )
                    })?
                    .1
                    .to_owned(),
            );
        } else if INF_RE.is_match(&line) {
            let line = line
                .split_once(':')
                .ok_or_else(|| {
                    anyhow!(
                        "Error parsing playlist '{}' line {linenum}: `#EXTINF:` cannot be empty",
                        file_path.as_ref().display()
                    )
                })?
                .1;

            let track_info = parse_extinf(line, linenum)?;

            if let Some(dur) = track_info.get("duration") {
                if let Ok(dur) = dur.parse() {
                    track_duration = Some(Duration::from_secs(dur));
                }
            }

            if let Some(year) = track_info.get("year") {
                if let Ok(year) = year.parse() {
                    track_year = Some(year);
                }
            }

            if let Some(number) = track_info.get("number") {
                if let Ok(number) = number.parse() {
                    track_number = Some(number);
                }
            }

            track_name = track_info.get("title").cloned();
        } else if COMMENT_RE.is_match(&line) {
            // do nothing
        } else {
            let track_path = line;
            tracks.push(Track {
                title: track_name.clone(),
                artist: track_artist
                    .clone()
                    .map_or_else(|| "Unknown".to_owned(), |artist| artist),
                album: track_album
                    .clone()
                    .map_or_else(|| "Unknown".to_owned(), |album| album),
                year: track_year,
                number: track_number,
                length: track_duration.map_or(Duration::ZERO, |length| length),
                file_path: track_path,
            });

            track_artist = None;
            track_album = None;
            track_duration = None;
            track_name = None;
            track_year = None;
            track_number = None;
        }
    }

    Ok((name, tracks))
}

impl Load for Playlist {
    /// Load playlist from an m3u8 file
    fn load<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let file = File::open(&file_path)?;
        let mut lines = BufReader::new(file).lines().enumerate();

        if let Some((_, Ok(line))) = lines.next() {
            check_header(&line, &file_path)?;
        } else {
            return Err(anyhow!(
                "{}: Invalid m3u8 file",
                file_path.as_ref().display()
            ));
        }

        let (mut name, tracks) = parse_lines(&mut lines, file_path)?;

        if name.is_empty() {
            "Untitled".clone_into(&mut name);
        }

        Ok(Self { name, tracks })
    }
}

fn parse_extinf(extinf: &str, linenum: usize) -> Result<HashMap<String, String>> {
    let mut track_info = HashMap::new();

    let (properties, title) = extinf.split_once(',').ok_or_else(|| {
        anyhow!(
            "Error parsing #EXTINF on line {linenum}: missing comma: '{}'",
            extinf
        )
    })?;
    track_info.insert("title".to_string(), title.to_string());
    let mut properties = properties.split(|c: char| c.is_whitespace());
    if let Some(duration) = properties.next() {
        track_info.insert("duration".to_string(), duration.to_string());
    }
    for p in properties {
        let (key, value) = p.split_once('=').ok_or_else(|| {
            anyhow!(
                "Error parsing #EXTINF on line {linenum}: property '{}' is not a key-value pair",
                p
            )
        })?;
        track_info.insert(key.to_string(), value.to_string());
    }

    Ok(track_info)
}
