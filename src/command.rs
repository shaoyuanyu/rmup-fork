/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::ui::ScreenEnum;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Command {
    Quit,
    Down,
    Up,
    NextPanel,
    PrevPanel,
    Play,
    Pause,
    Stop,
    TogglePlay,
    ToggleShuffle,
    ToggleRepeat,
    QueueAndPlay,
    GotoTop,
    GotoBottom,
    GotoScreen(ScreenEnum),
    NewPlaylist(Option<String>),
    PlaylistAdd,
    SelectPlaylist,
    PrevTrack,
    NextTrack,
    EnterCommand,
    AddPath(PathBuf),
    PlayTrack(PathBuf),
    Nop,
}

impl Command {
    pub fn parse(command: &str) -> Result<Self> {
        let mut tokens = command.split_whitespace();
        match tokens.next() {
            Some("q" | "quit" | "exit") => Ok(Self::Quit),
            Some("s" | "shuf" | "shuffle") => Ok(Self::ToggleShuffle),
            Some("r" | "rep" | "repeat") => Ok(Self::ToggleRepeat),
            Some("screen") => match tokens.next() {
                Some("1" | "main") => Ok(Self::GotoScreen(ScreenEnum::Main)),
                Some("2" | "playlist" | "playlists") => Ok(Self::GotoScreen(ScreenEnum::Playlists)),
                Some("0" | "help") => Ok(Self::GotoScreen(ScreenEnum::Help)),
                Some(other) => Err(anyhow!("screen: Invalid screen identifier: {}", other)),
                None => Err(anyhow!("screen: Missing argument SCREEN_ID")),
            },
            Some("h" | "help") => Ok(Self::GotoScreen(ScreenEnum::Help)),
            Some("a" | "add") => match command.split_once(' ') {
                Some((_, p)) => Ok(Self::AddPath(p.into())),
                None => Err(anyhow!("add: Missing argument PATH")),
            },
            Some("new-playlist" | "n") => match command.split_once(' ') {
                Some((_, name)) => Ok(Self::NewPlaylist(Some(name.into()))),
                None => Ok(Self::NewPlaylist(None)),
            },
            Some("play" | "p") => match command.split_once(' ') {
                Some((_, path)) => Ok(Self::PlayTrack(path.into())),
                None => Err(anyhow!("play: Missing argument PATH")),
            },
            Some(other) => Err(anyhow!("Invalid command: {}", other)),
            None => Ok(Self::Nop),
        }
    }
}
