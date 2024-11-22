/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

#![allow(
    clippy::enum_variant_names,
    clippy::module_name_repetitions,
    clippy::future_not_send
)]

use std::{
    collections::VecDeque,
    env, fs, io,
    path::Path,
    process,
    sync::Arc,
    time::{Duration, SystemTime},
};

use async_std::sync::Mutex;

use anyhow::{anyhow, Result};
use config::ConfOption;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use getopts::Options;
use media_system::MediaSystem;
use ratatui::{backend::CrosstermBackend, Terminal};

#[cfg(target_os = "linux")]
use mpris_server::Server;

mod command;
mod config;
mod library;
mod media_system;
mod playlist;
mod traits;
mod ui;
mod util;

#[cfg(target_os = "linux")]
mod mpris;

use library::{get_track_data, Library};
use traits::{Load, Save};
use ui::UI;

use command::Command::{
    AddPath, Down, EnterCommand, GotoBottom, GotoScreen, GotoTop, NewPlaylist, NextPanel,
    NextTrack, Nop, Pause, Play, PlayTrack, PlaylistAdd, PrevPanel, PrevTrack, QueueAndPlay, Quit,
    SelectPlaylist, Stop, TogglePlay, ToggleRepeat, ToggleShuffle, Up,
};
use ui::MovementDirection::{Bottom, Next, Prev, Top};

use crate::{command::Command, config::Config, media_system::MediaState, playlist::Playlist};

#[cfg(target_os = "linux")]
use crate::mpris::MprisPlayer;

pub enum Mode {
    Normal,
    PlaylistEntry,
    CommandEntry,
}

#[cfg(target_os = "linux")]
const BUS_NAME: &str = "xyz.jcheatum.RMuP";

#[async_std::main]
async fn main() -> Result<()> {
    let argv: Vec<String> = env::args().collect();
    let prog = &argv[0];
    let mut opts = Options::new();
    opts.optopt("c", "config", "Specify config file location", "FILE");
    opts.optopt("a", "add", "Add a directory to library", "DIR");
    opts.optopt("l", "lib", "Use the given library file", "FILE");
    opts.optflag("h", "help", "print usage and exit");
    let matches = match opts.parse(&argv[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("{prog}: Error: {f}");
            process::exit(1);
        }
    };
    if matches.opt_present("h") {
        print_usage(prog, &opts);
        process::exit(0);
    }

    let data_dir = dirs_next::data_dir()
        .expect("TODO: Handle other OSes")
        .join("rmup");
    if !data_dir.exists() {
        fs::create_dir(&data_dir)?;
    }

    let config_dir = dirs_next::config_dir()
        .expect("TODO: Handle other OSes")
        .join("rmup");
    if !config_dir.exists() {
        fs::create_dir(&config_dir)?;
    }

    let lib_file_path = data_dir.join("library.m3u8");
    let mut lib = if matches.opt_present("l") {
        let path = matches
            .opt_str("l")
            .ok_or_else(|| anyhow!("Option '-l' requires an argument"))?;
        Library::load(&path)?
    } else if lib_file_path.exists() {
        Library::load(&lib_file_path)?
    } else {
        Library::new()
    };
    lib.tracks.tracks.sort();

    if matches.opt_present("a") {
        let path = matches
            .opt_str("a")
            .ok_or_else(|| anyhow!("Option '-a' requires an argument"))?;
        lib.add_path(path)?;
        lib.tracks.tracks.sort();
        lib.save(&lib_file_path)?;
    }

    let config_file_path = config_dir.join("config.yaml");
    let config = if matches.opt_present("c") {
        let path = matches
            .opt_str("c")
            .ok_or_else(|| anyhow!("Option '-c' requires an argument"))?;
        Config::load(path)?
    } else if config_file_path.exists() {
        Config::load(config_file_path)?
    } else {
        let c = Config::default();
        c.save(config_file_path)?;
        c
    };

    let playlist_dir = data_dir.join("playlists");
    if !Path::new(&playlist_dir).exists() {
        fs::create_dir(&playlist_dir)?;
    }
    let playlists: Vec<Playlist> = fs::read_dir(&playlist_dir)?
        .filter_map(|entry| {
            entry.map_or(None, |entry| {
                entry.path().extension().and_then(|ext| match ext.to_str() {
                    Some("m3u8" | "m3u") => Some(entry.path()),
                    _ => None,
                })
            })
        })
        .filter_map(|p| Playlist::load(p).ok())
        .collect();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app_ui = UI::new(&lib, &config, &playlists);
    let state = Arc::new(Mutex::new(MediaState::default()));
    let command_queue = Arc::new(Mutex::new(VecDeque::<Command>::new()));
    #[cfg(target_os = "linux")]
    let server = Arc::new(Mutex::new(
        Server::new(
            BUS_NAME,
            MprisPlayer::new(command_queue.clone(), state.clone()),
        )
        .await?,
    ));
    let default_config = Config::default();
    let mut media_system = MediaSystem::new(
        #[cfg(target_os = "linux")]
        server,
        state,
        *config
            .options
            .get(&ConfOption::GaplessPlayback)
            .unwrap_or_else(|| {
                default_config
                    .options
                    .get(&ConfOption::GaplessPlayback)
                    .expect("Has default value")
            }),
    )
    .await?;

    let result: Result<()>;
    let poll_duration = Duration::from_millis(100);
    let mut time = SystemTime::now();
    let mut mode = Mode::Normal;

    loop {
        app_ui
            .draw(&mut terminal, media_system.state(), &config, &mode)
            .await?;

        if event::poll(poll_duration)? {
            if let Event::Key(ke) = event::read()? {
                if ke.kind == KeyEventKind::Press || ke.kind == KeyEventKind::Repeat {
                    match (&mode, ke.code) {
                        // Standard UI interaction
                        (Mode::Normal, _) => {
                            let mut guard = command_queue.lock().await;
                            guard.push_back(app_ui.get_key_command(ke, &config));
                        }

                        // Command/playlist entry
                        (Mode::PlaylistEntry, KeyCode::Enter) => {
                            let playlist_name = app_ui.command_line.get_contents();
                            let playlist = Playlist::new(&playlist_name);
                            app_ui.add_playlist(&playlist);
                            playlist.save(playlist_dir.join(format!("{}.m3u8", playlist.name)))?;
                            app_ui.command_line.reset();
                            mode = Mode::Normal;
                        }

                        (Mode::CommandEntry, KeyCode::Enter) => {
                            let command = app_ui.command_line.get_contents();
                            app_ui.command_line.reset();
                            match Command::parse(&command) {
                                Ok(cmd) => {
                                    let mut guard = command_queue.lock().await;
                                    guard.push_back(cmd);
                                }
                                Err(e) => {
                                    app_ui
                                        .command_line
                                        .textarea
                                        .insert_str(format!("{e}").as_str());
                                }
                            }
                            mode = Mode::Normal;
                        }

                        (Mode::PlaylistEntry | Mode::CommandEntry, KeyCode::Esc) => {
                            app_ui.command_line.reset();
                            mode = Mode::Normal;
                        }

                        (Mode::PlaylistEntry | Mode::CommandEntry, _) => {
                            app_ui.command_line.textarea.input(ke);
                        }
                    }
                }
            }
        }

        let mut guard = command_queue.lock().await;
        if let Some(cmd) = guard.pop_front() {
            drop(guard);
            match cmd {
                Quit => {
                    result = Ok(());
                    break;
                }
                Down => {
                    app_ui.switch_item(Next);
                    app_ui.update_lists();
                }
                Up => {
                    app_ui.switch_item(Prev);
                    app_ui.update_lists();
                }
                NextPanel => app_ui.switch_panel(Next),
                PrevPanel => app_ui.switch_panel(Prev),
                Play => {
                    media_system.play().await;
                    time = SystemTime::now();
                }
                Pause => {
                    media_system.pause().await;
                }
                Stop => {
                    media_system.stop().await?;
                    media_system.clear_queue();
                }
                TogglePlay => {
                    media_system.toggle_play().await;
                    time = SystemTime::now();
                }
                ToggleShuffle => media_system.toggle_shuffle().await,
                ToggleRepeat => media_system.toggle_repeat().await,
                QueueAndPlay => {
                    let queueable = app_ui.get_selected(false);
                    media_system.enqueue_and_play(&queueable).await?;
                    time = SystemTime::now();
                }
                GotoTop => app_ui.switch_item(Top),
                GotoBottom => app_ui.switch_item(Bottom),
                GotoScreen(s) => app_ui.switch_screen(s),
                NewPlaylist(None) => {
                    mode = Mode::PlaylistEntry;
                    app_ui.command_line.clear_contents();
                    app_ui.command_line.set_prompt("New playlist: ");
                }
                NewPlaylist(Some(playlist_name)) => {
                    let playlist = Playlist::new(&playlist_name);
                    app_ui.add_playlist(&playlist);
                    playlist.save(playlist_dir.join(format!("{}.m3u8", playlist.name)))?;
                }
                PlaylistAdd => {
                    app_ui.add_selected_to_playlist();
                    if let Some(pl) = app_ui.selected_playlist() {
                        pl.save(playlist_dir.join(format!("{}.m3u8", pl.name)))?;
                    }
                }
                SelectPlaylist => app_ui.select_current_playlist(),
                PrevTrack => media_system.play_prev().await?,
                NextTrack => media_system.play_next(true).await?,
                EnterCommand => {
                    mode = Mode::CommandEntry;
                    app_ui.command_line.reset();
                    app_ui.command_line.set_prompt(":");
                }
                AddPath(p) => {
                    let mut l = app_ui.library.clone();
                    match l.add_path(p) {
                        Ok(()) => {
                            l.tracks.tracks.sort();
                            l.save(&lib_file_path)?;
                            app_ui.update_library(l);
                        }
                        Err(e) => {
                            app_ui
                                .command_line
                                .textarea
                                .insert_str(e.to_string().as_str());
                        }
                    }
                }
                PlayTrack(path) => {
                    let (track, _, _) = get_track_data(path)?;
                    media_system.play_track(&track, true).await?;
                    time = SystemTime::now();
                }
                Nop => {}
            }
        }

        if media_system.state().lock().await.playing {
            media_system.update_progress(time.elapsed()?).await;
            time = SystemTime::now();
        }

        let play_next_cond = if media_system.gapless_playback() {
            media_system.time_remaining().await < Duration::from_secs_f32(0.1)
        } else {
            media_system.sink_empty()
        };

        if play_next_cond && !media_system.queue_empty() {
            media_system.play_next(false).await?;
            time = SystemTime::now();
        }

        app_ui.update_lists();
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {program} [options]");
    print!("{}", opts.usage(&brief));
}
