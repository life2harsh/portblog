use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn render(source: &str, accent: Color, muted: Color) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current: Vec<Span<'static>> = Vec::new();
    let mut styles = vec![Style::default()];
    let mut prefix: Vec<String> = Vec::new();
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut in_code = false;
    let mut link: Option<String> = None;

    let flush =
        |lines: &mut Vec<Line<'static>>, current: &mut Vec<Span<'static>>, prefix: &[String]| {
            if current.is_empty() {
                return;
            }
            let mut spans = vec![Span::styled(prefix.concat(), Style::default().fg(muted))];
            spans.append(current);
            lines.push(Line::from(spans));
        };
    let blank = |lines: &mut Vec<Line<'static>>| {
        if lines
            .last()
            .is_some_and(|line| !line.spans.iter().all(|span| span.content.trim().is_empty()))
        {
            lines.push(Line::default());
        }
    };

    for event in Parser::new_ext(source, options) {
        let style = *styles.last().unwrap_or(&Style::default());
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush(&mut lines, &mut current, &prefix);
                blank(&mut lines);
                let marks = match level {
                    HeadingLevel::H1 => "# ",
                    HeadingLevel::H2 => "## ",
                    _ => "### ",
                };
                current.push(Span::styled(marks, Style::default().fg(muted)));
                styles.push(Style::default().fg(accent).add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Heading(_)) => {
                styles.pop();
                flush(&mut lines, &mut current, &prefix);
                lines.push(Line::default());
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush(&mut lines, &mut current, &prefix);
                if lists.is_empty() {
                    lines.push(Line::default());
                }
            }
            Event::Start(Tag::Strong) => styles.push(style.add_modifier(Modifier::BOLD)),
            Event::Start(Tag::Emphasis) => styles.push(style.add_modifier(Modifier::ITALIC)),
            Event::Start(Tag::Strikethrough) => {
                styles.push(style.add_modifier(Modifier::CROSSED_OUT))
            }
            Event::End(TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough) => {
                styles.pop();
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                link = Some(dest_url.to_string());
                styles.push(style.add_modifier(Modifier::UNDERLINED));
            }
            Event::End(TagEnd::Link) => {
                styles.pop();
                if let Some(url) = link.take() {
                    current.push(Span::styled(
                        format!(" ({url})"),
                        Style::default().fg(muted),
                    ));
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                flush(&mut lines, &mut current, &prefix);
                prefix.push("│ ".into());
                styles.push(style.fg(muted).add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                flush(&mut lines, &mut current, &prefix);
                prefix.pop();
                styles.pop();
            }
            Event::Start(Tag::CodeBlock(_)) => {
                flush(&mut lines, &mut current, &prefix);
                in_code = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                lines.push(Line::default());
            }
            Event::Start(Tag::List(start)) => {
                flush(&mut lines, &mut current, &prefix);
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                flush(&mut lines, &mut current, &prefix);
                lists.pop();
                if lists.is_empty() {
                    lines.push(Line::default());
                }
            }
            Event::Start(Tag::Item) => {
                flush(&mut lines, &mut current, &prefix);
                let depth = lists.len().saturating_sub(1);
                let marker = match lists.last_mut() {
                    Some(Some(number)) => {
                        let label = format!("{number}. ");
                        *number += 1;
                        label
                    }
                    _ => "• ".into(),
                };
                current.push(Span::raw("  ".repeat(depth)));
                current.push(Span::styled(marker, Style::default().fg(accent)));
            }
            Event::End(TagEnd::Item) => flush(&mut lines, &mut current, &prefix),
            Event::Text(text) if in_code => {
                for code in text.lines() {
                    lines.push(Line::from(vec![
                        Span::styled(format!("{}│ ", prefix.concat()), Style::default().fg(muted)),
                        Span::styled(code.to_string(), Style::default().fg(accent)),
                    ]));
                }
            }
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                current.push(Span::styled(text.to_string(), style));
            }
            Event::Code(code) => current.push(Span::styled(
                code.to_string(),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            )),
            Event::SoftBreak => current.push(Span::raw(" ")),
            Event::HardBreak => flush(&mut lines, &mut current, &prefix),
            Event::Rule => {
                flush(&mut lines, &mut current, &prefix);
                lines.push(Line::styled("────────", Style::default().fg(muted)));
                lines.push(Line::default());
            }
            Event::TaskListMarker(done) => current.push(Span::styled(
                if done { "[x] " } else { "[ ] " },
                Style::default().fg(accent),
            )),
            _ => {}
        }
    }
    flush(&mut lines, &mut current, &prefix);
    while lines.last().is_some_and(|line| line.spans.is_empty()) {
        lines.pop();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(lines: &[Line<'_>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn renders_structure() {
        let lines = render(
            "# Title\n\nsome **bold** text\n\n- one\n- two\n\n1. a\n2. b\n\n> quoted\n\n```\ncode\n```",
            Color::Green,
            Color::Gray,
        );
        let plain = text(&lines);
        assert!(plain.contains(&"# Title".to_string()));
        assert!(plain.contains(&"some bold text".to_string()));
        assert!(plain.contains(&"• one".to_string()));
        assert!(plain.contains(&"2. b".to_string()));
        assert!(plain.contains(&"│ quoted".to_string()));
        assert!(plain.contains(&"│ code".to_string()));
    }

    #[test]
    fn links_show_destination() {
        let plain = text(&render(
            "[site](https://life2harsh.xyz)",
            Color::Green,
            Color::Gray,
        ));
        assert_eq!(plain[0], "site (https://life2harsh.xyz)");
    }
}
