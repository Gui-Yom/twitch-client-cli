use std::cmp::Reverse;
use std::io;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::ExecutableCommand;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use reqwest::Client;
use tokio::fs;
use tokio::time::sleep;
use tui::backend::CrosstermBackend;
use tui::Terminal;
use tui::widgets::{List, ListItem, ListState};

use stream::StreamEntry;

mod stream;
mod twitch;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<()> {
    let user = std::env::args()
        .nth(1)
        .expect("Expected a username to fetch follows");

    // Setup the terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let http_client = reqwest::Client::builder()
        .user_agent("No worries, it's for personal usage only")
        .build()?;

    let scrap_client_id = false;

    let client_id = if scrap_client_id {
        twitch::extract_client_id(&http_client).await?
    } else {
        // This id is always the same on the frontend
        twitch::TWITCH_CLIENT_ID.to_string()
    };

    let mut stream_entries = get_streams(&http_client, &client_id, &user).await?;

    if !stream_entries.is_empty() {
        stream_entries.sort_unstable_by_key(|it| Reverse(it.viewers));

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        loop {
            // Terminal events
            if event::poll(Duration::from_millis(1000))? {
                match event::read()? {
                    Event::Key(KeyEvent { code, .. }) => {
                        match code {
                            KeyCode::Enter => {
                                // PLay currently selected stream
                                play_stream(&http_client, &client_id, &stream_entries[list_state.selected().unwrap()].name).await?;
                            }
                            KeyCode::Up => {
                                // Move selection up
                                let selected = list_state.selected().unwrap();
                                if selected <= 0 {
                                    list_state.select(Some(stream_entries.len() - 1))
                                } else {
                                    list_state.select(Some(selected - 1))
                                }
                            }
                            KeyCode::Down => {
                                // Move selection down
                                let selected = list_state.selected().unwrap();
                                if selected >= stream_entries.len() - 1 {
                                    list_state.select(Some(0))
                                } else {
                                    list_state.select(Some(selected + 1))
                                }
                            }
                            KeyCode::Esc => {
                                break;
                            }
                            KeyCode::Char('r') => {
                                // Refresh
                                stream_entries = get_streams(&http_client, &client_id, &user).await?;
                                if stream_entries.is_empty() {
                                    println!("No online stream right now.");
                                    sleep(Duration::from_millis(2000)).await;
                                    break;
                                }
                                stream_entries.sort_unstable_by_key(|it| Reverse(it.viewers));
                                list_state.select(Some(0));
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            terminal.draw(|f| {
                let size = f.size();
                let items: Vec<ListItem> = stream_entries.iter().map(|it| ListItem::new(it)).collect();
                let list = List::new(items).highlight_symbol(">");
                f.render_stateful_widget(list, size, &mut list_state);
            })?;
        }
    } else {
        println!("No online stream right now.");
        sleep(Duration::from_millis(2000)).await;
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn get_streams(http_client: &Client, client_id: &str, user: &str) -> Result<Vec<StreamEntry>> {
    Ok(twitch::execute_main_query(&http_client, &client_id, &user, Some(100), None)
        .await?
        .data.unwrap()
        .user.unwrap()
        .follows.unwrap()
        .edges.unwrap()
        .into_iter()
        .filter_map(|it| {
            let node = it?.node?;
            //println!("{:#?}", node);
            if node.stream.is_none() {
                return None;
            }
            let stream = node.stream?;
            let channel = node.channel?;

            Some(StreamEntry {
                title: node.broadcast_settings?.title,
                name: channel.name,
                display_name: channel.display_name?,
                viewers: stream.viewers_count? as u32,
                game: stream.game?.display_name,
                best_video_settings: format!("{}p{}", stream.height?, stream.average_fps?),
                stream_type: stream.type_?,
            })
        })
        .collect())
}

async fn play_stream(http_client: &Client, client_id: &str, channel: &str) -> Result<()> {
    let token = twitch::get_stream_playback_token(&http_client, &client_id, channel).await?;
    let m3u8 = twitch::usher_get_hls_playlist(&http_client, channel, &token).await?;

    let file = format!("{}.m3u8", channel);
    let mut temp_file = std::env::temp_dir();
    temp_file.push(&file);
    fs::write(&temp_file, m3u8.as_bytes()).await?;

    let _vlc = tokio::process::Command::new("C:\\Program Files\\VideoLAN\\VLC\\vlc.exe")
        .arg(&temp_file)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false)
        .status()
        .await?;

    fs::remove_file(&temp_file).await?;
    Ok(())
}
