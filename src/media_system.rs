/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{collections::VecDeque, fs::File, io::BufReader, mem, sync::Arc, time::Duration};

use async_std::sync::Mutex;

use crate::{
    library::{album::Album, artist::Artist, track::Track},
    playlist::Playlist,
};
use anyhow::Result;
use rand::prelude::*;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

#[cfg(target_os = "linux")]
use crate::mpris::MprisPlayer;
#[cfg(target_os = "linux")]
use mpris_server::{LoopStatus, Metadata, PlaybackStatus, Property, Server, Time};

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum Repeat {
    On,
    Off,
    One,
}

#[derive(Default)]
pub struct MediaState {
    pub current_track: Option<Track>,
    pub current_track_progress: Option<Duration>,
    pub playing: bool,
    pub stopped: bool,
    pub shuffle: bool,
    pub repeat: Repeat,
}

pub struct MediaSystem {
    state: Arc<Mutex<MediaState>>,
    #[cfg(target_os = "linux")]
    mpris_server: Arc<Mutex<Server<MprisPlayer>>>,
    sink: Sink,
    stream_handle: OutputStreamHandle,
    _stream: OutputStream,
    queue: VecDeque<Track>,
    ordered_queue: VecDeque<Track>,
    history: Vec<Track>,
    gapless_playback: bool,
}

#[derive(Debug, Clone)]
pub enum Queueable {
    Artist(Artist),
    Album(Album),
    Playlist(Playlist),
    TrackList(Arc<[Track]>),
    Empty,
}

impl Queueable {
    pub fn get_tracks(&self) -> Vec<Track> {
        match self {
            Self::Artist(artist) => artist.albums[0].tracks.clone(),
            Self::Album(album) => album.tracks.clone(),
            Self::Playlist(playlist) => playlist.tracks.clone(),
            Self::TrackList(track_list) => Vec::from(track_list.as_ref()),
            Self::Empty => Vec::new(),
        }
    }
}

impl Default for Repeat {
    fn default() -> Self {
        Self::Off
    }
}

impl MediaSystem {
    pub async fn new(
        #[cfg(target_os = "linux")] mpris_server: Arc<Mutex<Server<MprisPlayer>>>,
        state: Arc<Mutex<MediaState>>,
        gapless_playback: bool,
    ) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            mpris_server
                .lock()
                .await
                .properties_changed([
                    Property::CanSeek(false),
                    Property::Metadata(Metadata::new()),
                    Property::PlaybackStatus(PlaybackStatus::Stopped),
                    Property::LoopStatus(LoopStatus::None),
                    Property::Shuffle(false),
                ])
                .await?;
        }

        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;

        #[allow(clippy::used_underscore_binding)]
        Ok(Self {
            state,
            #[cfg(target_os = "linux")]
            mpris_server,
            stream_handle,
            sink,
            _stream,
            queue: VecDeque::new(),
            ordered_queue: VecDeque::new(),
            history: Vec::new(),
            gapless_playback,
        })
    }

    /// Get the current state of the `MediaSystem`
    pub const fn state(&self) -> &Arc<Mutex<MediaState>> {
        &self.state
    }

    /// Add a track to the play queue
    pub fn enqueue(&mut self, track: &Track) {
        self.queue.push_back(track.clone());
        self.ordered_queue.push_back(track.clone());
    }

    /// If there is a current track and it is paused, resume it. Otherwise does
    /// nothing.
    pub async fn play(&self) {
        let mut guard = self.state.lock().await;
        if guard.current_track.is_some() && !guard.playing {
            guard.playing = true;
            guard.stopped = false;
            self.sink.play();
        }
        drop(guard);
        #[cfg(target_os = "linux")]
        {
            let _ = self
                .mpris_server
                .lock()
                .await
                .properties_changed([Property::PlaybackStatus(PlaybackStatus::Playing)])
                .await;
        }
    }

    /// If there is a current track and it is playing, pause it. Otherwise does
    /// nothing.
    pub async fn pause(&self) {
        let mut guard = self.state.lock().await;
        if guard.current_track.is_some() && guard.playing {
            guard.playing = false;
            self.sink.pause();
        }
        drop(guard);
        #[cfg(target_os = "linux")]
        {
            let _ = self
                .mpris_server
                .lock()
                .await
                .properties_changed([Property::PlaybackStatus(PlaybackStatus::Paused)])
                .await;
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        if !self.sink.empty() {
            self.sink.stop();
            self.state.lock().await.stopped = true;
            self.sink = Sink::try_new(&self.stream_handle)?;
        }

        #[cfg(target_os = "linux")]
        {
            self.mpris_server
                .lock()
                .await
                .properties_changed([Property::PlaybackStatus(PlaybackStatus::Stopped)])
                .await?;
        }

        Ok(())
    }

    pub async fn play_track(&mut self, track: &Track, interrupt: bool) -> Result<()> {
        if interrupt {
            self.stop().await?;
        }

        let file = BufReader::new(File::open(&track.file_path)?);
        let source = Decoder::new(file)?;
        let mut guard = self.state.lock().await;

        guard.current_track = Some(track.clone());
        guard.current_track_progress = Some(Duration::from_millis(0));
        guard.playing = true;
        drop(guard);
        self.sink.append(source);

        #[allow(clippy::cast_possible_wrap)]
        #[cfg(target_os = "linux")]
        {
            let mut metadata_builder = Metadata::builder()
                .title(
                    track
                        .title
                        .clone()
                        .unwrap_or_else(|| track.file_path.clone()),
                )
                .artist([&track.artist])
                .album(&track.album)
                .length(Time::from_secs(track.length.as_secs() as i64));
            if let Some(number) = track.number {
                metadata_builder = metadata_builder.track_number(number as i32);
            }
            let _ = self
                .mpris_server
                .lock()
                .await
                .properties_changed([
                    Property::PlaybackStatus(PlaybackStatus::Playing),
                    Property::Metadata(metadata_builder.build()),
                ])
                .await;
        }
        Ok(())
    }

    /// Play the next track in the queue
    pub async fn play_next(&mut self, interrupt: bool) -> Result<()> {
        let mut guard = self.state.lock().await;

        let next_track = if guard.repeat == Repeat::One {
            guard.current_track.clone()
        } else {
            self.queue.pop_front()
        };

        if let Some(track) = next_track {
            if let Some(current_track) = mem::take(&mut guard.current_track) {
                self.history.push(current_track.clone());
                if guard.repeat == Repeat::On {
                    self.queue.push_back(current_track);
                }
            }
            drop(guard);
            self.play_track(&track, interrupt || !self.gapless_playback)
                .await?;
        }

        Ok(())
    }

    pub async fn play_prev(&mut self) -> Result<()> {
        let guard = self.state.lock().await;

        if let Some(prev_track) = self.history.pop() {
            if let Some(current_track) = &guard.current_track {
                self.queue.push_front(current_track.clone());
            }
            drop(guard);
            self.play_track(&prev_track, true).await?;
        } else if let Some(current_track) = guard.current_track.clone() {
            drop(guard);
            self.play_track(&current_track, true).await?;
        }

        Ok(())
    }

    pub async fn enqueue_and_play(&mut self, queueable: &Queueable) -> Result<()> {
        self.queue.clear();
        let tracks = queueable.get_tracks();
        for t in tracks {
            self.enqueue(&t);
        }
        match queueable {
            Queueable::Artist(_) | Queueable::Album(_) | Queueable::Playlist(_) => {
                if self.state.lock().await.shuffle {
                    self.queue
                        .make_contiguous()
                        .shuffle(&mut rand::thread_rng());
                }

                if let Some(track) = self.queue.pop_front() {
                    self.play_track(&track, true).await
                } else {
                    Ok(())
                }
            }
            Queueable::TrackList(_) => {
                if let Some(track) = self.queue.pop_front() {
                    self.play_track(&track, true).await?;
                }

                if self.state.lock().await.shuffle {
                    self.queue
                        .make_contiguous()
                        .shuffle(&mut rand::thread_rng());
                }

                Ok(())
            }
            Queueable::Empty => Ok(()),
        }
    }

    /// Add the given duration to the current track's playback progress
    pub async fn update_progress(&self, duration: Duration) {
        let mut guard = self.state.lock().await;

        if let Some(progress) = guard.current_track_progress.as_mut() {
            *progress += duration;
        }

        if self.sink_empty() {
            guard.playing = false;
            guard.current_track_progress = None;
            if guard.repeat != Repeat::One {
                guard.current_track = None;
            }
        }
    }

    /// Toggle between playing/paused
    pub async fn toggle_play(&self) {
        let guard = self.state.lock().await;
        let status = if guard.current_track.is_some() {
            if guard.playing {
                drop(guard);
                self.pause().await;
                #[cfg(target_os = "linux")]
                PlaybackStatus::Paused
            } else {
                drop(guard);
                self.play().await;
                #[cfg(target_os = "linux")]
                PlaybackStatus::Playing
            }
        } else {
            #[cfg(target_os = "linux")]
            PlaybackStatus::Stopped
        };
        #[cfg(target_os = "linux")]
        {
            let _ = self
                .mpris_server
                .lock()
                .await
                .properties_changed([Property::PlaybackStatus(status)])
                .await;
        }
    }

    pub async fn toggle_shuffle(&mut self) {
        let mut guard = self.state.lock().await;
        guard.shuffle = !guard.shuffle;

        #[cfg(target_os = "linux")]
        {
            let _ = self
                .mpris_server
                .lock()
                .await
                .properties_changed([Property::Shuffle(guard.shuffle)])
                .await;
        }

        if guard.shuffle {
            drop(guard);
            self.queue
                .make_contiguous()
                .shuffle(&mut rand::thread_rng());
        } else {
            let current_track = guard.current_track.clone();
            drop(guard);
            self.queue.clone_from(&self.ordered_queue);
            let current_track_index = self
                .queue
                .iter()
                .position(|t| Some(t) == current_track.as_ref());
            if let Some(index) = current_track_index {
                self.queue.rotate_left(index + 1);
            }
        }
    }

    pub async fn toggle_repeat(&self) {
        use Repeat::{Off, On, One};
        let mut guard = self.state.lock().await;

        guard.repeat = match guard.repeat {
            Off => One,
            One => On,
            On => Off,
        };

        #[cfg(target_os = "linux")]
        {
            let _ = self
                .mpris_server
                .lock()
                .await
                .properties_changed([Property::LoopStatus(guard.repeat.into())])
                .await;
            drop(guard);
        }
    }

    pub fn sink_empty(&self) -> bool {
        self.sink.empty()
    }

    pub async fn time_remaining(&self) -> Duration {
        let guard = self.state.lock().await;
        guard
            .current_track
            .as_ref()
            .map_or(Duration::ZERO, |current_track| {
                guard
                    .current_track_progress
                    .map_or(Duration::ZERO, |current_track_progress| {
                        current_track
                            .length
                            .checked_sub(current_track_progress)
                            .unwrap_or(Duration::ZERO)
                    })
            })
    }

    pub fn queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
    }

    pub const fn gapless_playback(&self) -> bool {
        self.gapless_playback
    }
}

#[cfg(target_os = "linux")]
impl From<Repeat> for LoopStatus {
    fn from(val: Repeat) -> Self {
        match val {
            Repeat::On => Self::Playlist,
            Repeat::Off => Self::None,
            Repeat::One => Self::Track,
        }
    }
}
