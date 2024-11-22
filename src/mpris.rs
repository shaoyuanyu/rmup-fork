/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#![allow(clippy::cast_possible_wrap)]

use std::{collections::VecDeque, sync::Arc};

use async_std::sync::Mutex;

use mpris_server::{
    zbus::{fdo, Result},
    LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, RootInterface, Time,
    TrackId, Volume,
};

use crate::{
    command::Command,
    media_system::{MediaState, Repeat},
};

pub struct MprisPlayer {
    command_queue: Arc<Mutex<VecDeque<Command>>>,
    media_state: Arc<Mutex<MediaState>>,
}

impl MprisPlayer {
    pub const fn new(
        command_queue: Arc<Mutex<VecDeque<Command>>>,
        media_state: Arc<Mutex<MediaState>>,
    ) -> Self {
        Self {
            command_queue,
            media_state,
        }
    }
}

impl RootInterface for MprisPlayer {
    async fn raise(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> fdo::Result<()> {
        self.command_queue.lock().await.push_back(Command::Quit);
        Ok(())
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, _fullscreen: bool) -> Result<()> {
        Ok(())
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn identity(&self) -> fdo::Result<String> {
        Ok("RMuP".to_string())
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok("rmup".to_string())
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![
            "audio/mpeg".into(),
            "audio/flac".into(),
            "audio/x-flac".into(),
            "audio/aiff".into(),
            "audio/x-aiff".into(),
            "audio/ogg".into(),
            "audio/opus".into(),
            "audio/aac".into(),
            "audio/wav".into(),
            "audio/vnd.wav".into(),
        ])
    }
}

impl PlayerInterface for MprisPlayer {
    async fn next(&self) -> fdo::Result<()> {
        self.command_queue
            .lock()
            .await
            .push_back(Command::NextTrack);
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        self.command_queue
            .lock()
            .await
            .push_back(Command::PrevTrack);
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.command_queue.lock().await.push_back(Command::Pause);
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        self.command_queue
            .lock()
            .await
            .push_back(Command::TogglePlay);
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        self.command_queue.lock().await.push_back(Command::Stop);
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        self.command_queue.lock().await.push_back(Command::Play);
        Ok(())
    }

    async fn seek(&self, _offset: Time) -> fdo::Result<()> {
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, _position: Time) -> fdo::Result<()> {
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Ok(())
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        let guard = self.media_state.lock().await;

        Ok(if guard.playing {
            PlaybackStatus::Playing
        } else if guard.stopped {
            PlaybackStatus::Stopped
        } else {
            PlaybackStatus::Paused
        })
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(LoopStatus::None)
    }

    async fn set_loop_status(&self, loop_status: LoopStatus) -> Result<()> {
        let mut guard = self.media_state.lock().await;
        match loop_status {
            LoopStatus::None => guard.repeat = Repeat::Off,
            LoopStatus::Track => guard.repeat = Repeat::One,
            LoopStatus::Playlist => guard.repeat = Repeat::On,
        }
        drop(guard);
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(PlaybackRate::default())
    }

    async fn set_rate(&self, _rate: PlaybackRate) -> Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        let guard = self.media_state.lock().await;
        Ok(guard.shuffle)
    }

    async fn set_shuffle(&self, shuffle: bool) -> Result<()> {
        self.media_state.lock().await.shuffle = shuffle;
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        let metadata = self
            .media_state
            .lock()
            .await
            .current_track
            .as_ref()
            .map_or_else(Metadata::default, |track| {
                let mut builder = Metadata::builder()
                    .artist([&track.artist])
                    .album(&track.album)
                    .title(
                        track
                            .title
                            .clone()
                            .unwrap_or_else(|| track.file_path.clone()),
                    )
                    .length(Time::from_secs(track.length.as_secs() as i64));
                if let Some(number) = track.number {
                    builder = builder.track_number(number as i32);
                }
                builder.build()
            });

        Ok(metadata)
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(Volume::default())
    }

    async fn set_volume(&self, _volume: Volume) -> Result<()> {
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        let pos = self
            .media_state
            .lock()
            .await
            .current_track_progress
            .map_or(Time::ZERO, |position| {
                Time::from_secs(position.as_secs() as i64)
            });
        Ok(pos)
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(PlaybackRate::default())
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(PlaybackRate::default())
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}
