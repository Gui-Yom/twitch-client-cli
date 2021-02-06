use std::fmt;
use std::fmt::{Display, Formatter};

use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};

#[derive(Debug)]
pub struct StreamEntry {
    pub title: String,
    pub name: String,
    pub display_name: String,
    pub viewers: u32,
    pub game: String,
    pub best_video_settings: String,
    pub stream_type: String,
}

impl<'a> From<&'a StreamEntry> for Text<'a> {
    fn from(stream: &'a StreamEntry) -> Self {
        Text::from(vec![
            Spans(vec![
                Span::styled(&stream.display_name, Style::default().fg(Color::Blue)),
                Span::raw(" - "),
                Span::styled(&stream.title, Style::default().add_modifier(Modifier::ITALIC)),
            ]),
            Spans(vec![
                Span::styled(&stream.stream_type, Style::default().fg(Color::Green)),
                Span::raw(" in "),
                Span::styled(&stream.game, Style::default().fg(Color::Gray)),
                Span::raw(format!(" | {} viewers | ", stream.viewers)),
                Span::raw(&stream.best_video_settings)
            ])
        ])
    }
}
