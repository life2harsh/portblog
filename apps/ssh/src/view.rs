use content::{EXPERIENCES, PROJECTS, format_date};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::{App, Tab};
use crate::journal::Status;
use crate::markdown;

const MUTED: Color = Color::Gray;
const FAINT: Color = Color::DarkGray;
const ON_ACCENT: Color = Color::Black;
const HEADLINE: &str = "i make software and other things computers probably didn’t ask for.";

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if area.width < 40 || area.height < 12 {
        frame.render_widget(
            Paragraph::new("make the terminal a little bigger (40×12)")
                .alignment(Alignment::Center),
            area,
        );
        return;
    }
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    draw_header(frame, app, header);
    if app.reading {
        draw_article(frame, app, body);
    } else if body.width >= 96 && body.height >= 22 {
        draw_wide(frame, app, body);
    } else {
        draw_narrow(frame, app, body);
    }
    draw_footer(frame, app, footer);
    if app.help {
        draw_help(frame, app, area);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let accent = app.accent();
    let mut spans = vec![
        Span::styled(
            " life2harsh.xyz",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled("    ", Style::default().fg(FAINT)),
    ];
    for (index, tab) in Tab::ALL.iter().enumerate() {
        let label = format!(" {} {} ", index + 1, tab.label());
        let style = if *tab == app.tab {
            Style::default().bg(accent).fg(ON_ACCENT)
        } else {
            Style::default().fg(MUTED)
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
    }
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(FAINT));
    frame.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
    if area.width > 70 {
        let tag = Paragraph::new(Span::styled(
            "ssh.life2harsh.xyz ",
            Style::default().fg(FAINT),
        ))
        .alignment(Alignment::Right);
        frame.render_widget(tag, Rect { height: 1, ..area });
    }
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let hints: &[(&str, &str)] = if app.reading {
        &[
            ("j k", "scroll"),
            ("esc", "back"),
            ("q", "quit"),
            ("space", "page"),
            ("t", "theme"),
        ]
    } else {
        &[
            ("j k", "move"),
            ("q", "quit"),
            ("1 2 3", "tabs"),
            ("enter", "read"),
            ("?", "help"),
            ("t", "theme"),
        ]
    };
    let room = if area.width > 80 {
        hints.len()
    } else {
        (area.width as usize / 13).max(2)
    };
    let mut spans = vec![Span::raw(" ")];
    for (key, label) in hints.iter().take(room) {
        spans.push(Span::styled(
            *key,
            Style::default().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {label}   "),
            Style::default().fg(MUTED),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
    if area.width > 80 {
        let theme = Paragraph::new(Line::from(vec![
            Span::styled("THEME ", Style::default().fg(MUTED)),
            Span::styled(
                format!("{} ", app.theme_label()),
                Style::default().fg(app.accent()),
            ),
        ]))
        .alignment(Alignment::Right);
        frame.render_widget(theme, area);
    }
}

fn draw_wide(frame: &mut Frame, app: &mut App, area: Rect) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(area);
    let divider = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(FAINT));
    let inner_left = divider.inner(left);
    frame.render_widget(divider, left);
    let [top, bottom] = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
        .areas(inner_left);
    let [identity, art] =
        Layout::horizontal([Constraint::Percentage(46), Constraint::Percentage(54)]).areas(top);
    draw_identity(frame, app, pad(identity, 2, 1));
    draw_art(frame, app, art);
    let rule = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(FAINT));
    let detail = rule.inner(bottom);
    frame.render_widget(
        rule,
        Rect {
            x: bottom.x + 2,
            width: bottom.width.saturating_sub(4),
            ..bottom
        },
    );
    draw_detail(frame, app, pad(detail, 2, 1));
    draw_list(frame, app, pad(right, 2, 1));
}

fn draw_narrow(frame: &mut Frame, app: &mut App, area: Rect) {
    let [list, detail] =
        Layout::vertical([Constraint::Percentage(48), Constraint::Percentage(52)]).areas(area);
    draw_list(frame, app, pad(list, 1, 0));
    let rule = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(FAINT));
    let inner = rule.inner(detail);
    frame.render_widget(rule, detail);
    draw_detail(frame, app, pad(inner, 1, 0));
}

fn pad(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x + horizontal.min(area.width / 2),
        y: area.y + vertical.min(area.height / 2),
        width: area.width.saturating_sub(horizontal * 2),
        height: area.height.saturating_sub(vertical * 2),
    }
}

fn label(text: String) -> Line<'static> {
    Line::styled(text, Style::default().fg(MUTED))
}

fn key_value(key: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<10}"), Style::default().fg(MUTED)),
        Span::raw(value),
    ])
}

fn draw_identity(frame: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from(vec![
            Span::styled(HEADLINE, Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" █", Style::default().fg(app.accent())),
        ]),
        Line::default(),
        Line::raw("systems, interfaces, creative tools, and weird experiments."),
        Line::default(),
        key_value("LOCATION", "somewhere on earth".into()),
        key_value("STACK", "rust, web, systems".into()),
        key_value("STATUS", "searching for a W".into()),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
}

fn draw_art(frame: &mut Frame, app: &App, area: Rect) {
    if area.width < 10 || area.height < 5 {
        return;
    }
    let width = f64::from(area.width) * 2.0;
    let height = f64::from(area.height) * 4.0;
    let accent = app.accent();
    let rings = app.field.rings(width, height);
    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .x_bounds([0.0, width])
        .y_bounds([0.0, height])
        .paint(move |context| {
            for ring in rings.iter().step_by(7) {
                let color = if ring.mix > 0.55 {
                    accent
                } else if ring.depth > 0.5 {
                    MUTED
                } else {
                    FAINT
                };
                for pair in ring.points.windows(2) {
                    context.draw(&CanvasLine {
                        x1: pair[0].0,
                        y1: height - pair[0].1,
                        x2: pair[1].0,
                        y2: height - pair[1].1,
                        color,
                    });
                }
            }
        });
    frame.render_widget(canvas, area);
}

fn draw_detail(frame: &mut Frame, app: &App, area: Rect) {
    let accent = app.accent();
    let heading = |text: String| {
        Line::styled(
            text,
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )
    };
    let lines: Vec<Line<'static>> = match app.tab {
        Tab::About => {
            let experience = &EXPERIENCES[app.experience];
            vec![
                label("ABOUT".into()),
                heading("harsh jha".into()),
                Line::raw(
                    "Developer, student and epistemically curious. I like terminal tools, systems software, WebAssembly and anything that turns math into pictures.",
                ),
                Line::default(),
                label("EXPERIENCE".into()),
                Line::styled(
                    format!("{} — {}", experience.organization, experience.role),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Line::styled(experience.period.to_string(), Style::default().fg(MUTED)),
                Line::raw(experience.description.to_string()),
                Line::default(),
                Line::styled(
                    "github.com/life2harsh   linkedin.com/in/life2harsh   x.com/life2harsh",
                    Style::default().fg(MUTED),
                ),
                Line::styled(
                    "blog.life2harsh.xyz   jhaharsh29@gmail.com",
                    Style::default().fg(MUTED),
                ),
            ]
        }
        Tab::Projects => {
            let project = &PROJECTS[app.project];
            let mut lines = vec![
                label("SELECTED PROJECT".into()),
                heading(project.name.to_lowercase()),
                Line::raw(project.description.to_string()),
                Line::default(),
                key_value("YEAR", project.year.to_string()),
                key_value("STATUS", project.status.label().to_string()),
                key_value("STACK", project.stack.join(", ")),
                Line::default(),
            ];
            if let Some(long) = project.long_description {
                lines.insert(3, Line::raw(long.to_string()));
            }
            for link in project.links {
                lines.push(Line::from(vec![
                    Span::styled("→ ", Style::default().fg(accent)),
                    Span::styled(
                        link.url.trim_start_matches("https://").to_string(),
                        Style::default().add_modifier(Modifier::UNDERLINED),
                    ),
                ]));
            }
            if let Some(live) = project.live_url {
                lines.push(Line::from(vec![
                    Span::styled("→ ", Style::default().fg(accent)),
                    Span::styled(
                        live.trim_start_matches("https://").to_string(),
                        Style::default().add_modifier(Modifier::UNDERLINED),
                    ),
                ]));
            }
            lines
        }
        Tab::Writing => match (app.selected_entry(), app.status) {
            (Some(entry), _) => vec![
                label(format!(
                    "{}, {} MIN READ",
                    format_date(entry.created_at),
                    entry.reading_minutes()
                )),
                heading(entry.heading()),
                Line::default(),
                Line::raw(entry.description(420)),
                Line::default(),
                Line::from(vec![
                    Span::styled("enter", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(" to read", Style::default().fg(MUTED)),
                ]),
            ],
            (None, Status::Loading) => {
                vec![label("LOADING".into()), Line::raw("pulling posts ░▒▓")]
            }
            (None, Status::Offline) => vec![
                label("OFFLINE".into()),
                Line::raw("the journal is unreachable right now. read it at blog.life2harsh.xyz."),
            ],
            (None, Status::Ready) => {
                vec![
                    label("EMPTY".into()),
                    Line::raw("nothing here yet. check back soon."),
                ]
            }
        },
    };
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
}

fn draw_list(frame: &mut Frame, app: &App, area: Rect) {
    let accent = app.accent();
    let (title, rows, selected): (&str, Vec<(String, String, String)>, usize) = match app.tab {
        Tab::About => (
            "EXPERIENCE",
            EXPERIENCES
                .iter()
                .map(|item| {
                    (
                        item.organization.to_string(),
                        item.role.to_string(),
                        item.period.to_string(),
                    )
                })
                .collect(),
            app.experience,
        ),
        Tab::Projects => (
            "PROJECTS",
            PROJECTS
                .iter()
                .map(|project| {
                    (
                        project.name.to_lowercase(),
                        project
                            .stack
                            .iter()
                            .take(3)
                            .map(|item| item.to_lowercase())
                            .collect::<Vec<_>>()
                            .join(", "),
                        project.year.to_string(),
                    )
                })
                .collect(),
            app.project,
        ),
        Tab::Writing => (
            "POSTS",
            app.entries
                .iter()
                .map(|entry| {
                    (
                        entry.heading(),
                        entry.tags.join(", "),
                        format_date(entry.created_at),
                    )
                })
                .collect(),
            app.entry,
        ),
    };
    let [head, list] = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
    let header = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(FAINT));
    frame.render_widget(
        Paragraph::new(Line::styled(
            title,
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .block(header),
        head,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("{:02} ITEMS", rows.len()),
            Style::default().fg(MUTED),
        ))
        .alignment(Alignment::Right),
        Rect { height: 1, ..head },
    );
    let width = list.width as usize;
    let items = rows
        .iter()
        .enumerate()
        .map(|(index, (name, meta, aside))| {
            let room = width.saturating_sub(aside.chars().count() + 9);
            let name: String = name.chars().take(room).collect();
            let gap = width.saturating_sub(name.chars().count() + aside.chars().count() + 6);
            ListItem::new(vec![
                Line::default(),
                Line::from(vec![
                    Span::styled(format!(" {:02}  ", index + 1), Style::default().fg(MUTED)),
                    Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" ".repeat(gap)),
                    Span::styled(aside.clone(), Style::default().fg(MUTED)),
                ]),
                Line::from(vec![
                    Span::raw("     "),
                    Span::styled(meta.clone(), Style::default().fg(MUTED)),
                ]),
            ])
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected((!rows.is_empty()).then_some(selected));
    let widget = List::new(items).highlight_style(Style::default().bg(accent).fg(ON_ACCENT));
    frame.render_stateful_widget(widget, list, &mut state);
}

fn draw_article(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(entry) = app.selected_entry().cloned() else {
        return;
    };
    let accent = app.accent();
    let width = area.width.min(88);
    let column = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + 1,
        width,
        height: area.height.saturating_sub(1),
    };
    let mut lines = vec![label(format!(
        "{}, {} MIN READ",
        format_date(entry.created_at),
        entry.reading_minutes()
    ))];
    if let Some(title) = &entry.title {
        lines.push(Line::default());
        lines.push(Line::styled(
            title.clone(),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(summary) = &entry.summary {
        lines.push(Line::styled(
            summary.clone(),
            Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
        ));
    }
    if !entry.tags.is_empty() {
        lines.push(Line::styled(
            entry
                .tags
                .iter()
                .map(|tag| format!("[{tag}]"))
                .collect::<Vec<_>>()
                .join(" "),
            Style::default().fg(MUTED),
        ));
    }
    lines.push(Line::styled(
        "─".repeat(width as usize),
        Style::default().fg(FAINT),
    ));
    lines.push(Line::default());
    lines.extend(markdown::render(&entry.body, accent, MUTED));
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let total = paragraph.line_count(column.width) as u16;
    app.scroll = app
        .scroll
        .min(total.saturating_sub(column.height.saturating_sub(2)));
    frame.render_widget(paragraph.scroll((app.scroll, 0)), column);
}

fn draw_help(frame: &mut Frame, app: &App, area: Rect) {
    let width = 46.min(area.width.saturating_sub(4));
    let height = 16.min(area.height.saturating_sub(2));
    let rect = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };
    let rows = [
        ("1 2 3  a p w", "about, projects, writing"),
        ("tab  h l", "next / previous tab"),
        ("j k  ↑ ↓", "move, or scroll a post"),
        ("g G", "first / last"),
        ("enter", "read the selected post"),
        ("space  pgup pgdn", "page through a post"),
        ("esc", "back to the list"),
        ("t", "cycle accent theme"),
        ("q  ctrl-c", "leave"),
    ];
    let mut lines = vec![Line::default()];
    for (keys, action) in rows {
        lines.push(Line::from(vec![
            Span::styled(format!("  {keys:<18}"), Style::default().fg(app.accent())),
            Span::raw(action),
        ]));
    }
    lines.push(Line::default());
    lines.push(Line::styled(
        "  any key to close",
        Style::default().fg(MUTED),
    ));
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" keys ")
        .border_style(Style::default().fg(app.accent()));
    frame.render_widget(Clear, rect);
    frame.render_widget(Paragraph::new(lines).block(block), rect);
}
