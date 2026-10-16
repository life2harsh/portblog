use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd, html};

const VIDEO: [&str; 3] = [".mp4", ".webm", ".mov"];

pub fn parse_markdown(src: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let events = Parser::new_ext(src, options)
        .into_offset_iter()
        .collect::<Vec<_>>();
    let mut blocks = Vec::with_capacity(events.len());
    let mut index = 0;
    while index < events.len() {
        if let [
            (Event::Start(Tag::Paragraph), _),
            (Event::Code(_), range),
            (Event::End(TagEnd::Paragraph), _),
            ..,
        ] = &events[index..]
            && src[range.clone()].contains('\n')
        {
            let inner = src[range.clone()]
                .trim_matches('`')
                .trim_matches(['\n', '\r', ' ']);
            blocks.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(
                CowStr::Borrowed(""),
            ))));
            blocks.push(Event::Text(CowStr::from(format!("{inner}\n"))));
            blocks.push(Event::End(TagEnd::CodeBlock));
            index += 3;
            continue;
        }
        blocks.push(events[index].0.clone());
        index += 1;
    }

    let mut skipping = false;
    let parser = blocks.into_iter().filter_map(move |event| match event {
        Event::Start(Tag::Image { dest_url, .. }) if is_video(&dest_url) => {
            skipping = true;
            Some(Event::Html(CowStr::from(format!(
                "<video controls playsinline preload=\"metadata\" src=\"{}\"></video>",
                attribute(&dest_url)
            ))))
        }
        Event::End(TagEnd::Image) if skipping => {
            skipping = false;
            None
        }
        _ if skipping => None,
        Event::Html(raw) | Event::InlineHtml(raw) => Some(Event::Text(raw)),
        Event::SoftBreak => Some(Event::HardBreak),
        other => Some(other),
    });
    let mut html_output = String::with_capacity(src.len() * 2);
    html::push_html(&mut html_output, parser);
    html_output
}

fn is_video(url: &str) -> bool {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    VIDEO.iter().any(|extension| path.ends_with(extension))
}

fn attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::parse_markdown;

    #[test]
    fn raw_html_is_escaped() {
        let html = parse_markdown("hi <script>alert(1)</script>\n\n<img src=x onerror=alert(1)>");
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn markdown_still_renders() {
        let html = parse_markdown("# Title\n\n**bold** and `code`");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<code>code</code>"));
    }

    #[test]
    fn images_render() {
        let html = parse_markdown("![a cat](https://x.dev/media/cat.webp)");
        assert!(html.contains("<img src=\"https://x.dev/media/cat.webp\" alt=\"a cat\""));
    }

    #[test]
    fn video_links_become_players() {
        let html = parse_markdown("![demo](https://x.dev/media/demo.MP4?v=1)\n\nafter");
        assert!(html.contains("<video controls"));
        assert!(html.contains("src=\"https://x.dev/media/demo.MP4?v=1\""));
        assert!(!html.contains("demo</"));
        assert!(html.contains("<p>after</p>"));
    }

    #[test]
    fn multiline_backticks_become_a_block() {
        let html = parse_markdown("`mkdir -p ~/a\nadb pull /sdcard ~/a/`\n\nafter `inline` code");
        assert!(html.contains("<pre><code>mkdir -p ~/a\nadb pull /sdcard ~/a/\n</code></pre>"));
        assert!(html.contains("<p>after <code>inline</code> code</p>"));
    }

    #[test]
    fn fenced_blocks_still_render() {
        let html = parse_markdown("```sh\nls -la\n```");
        assert!(html.contains("<pre><code class=\"language-sh\">ls -la\n</code></pre>"));
    }

    #[test]
    fn single_newlines_are_kept() {
        let html = parse_markdown("enable it.\nmake a backup\n\nnext");
        assert!(html.contains("<p>enable it.<br />\nmake a backup</p>"));
    }

    #[test]
    fn arrows_stay_as_typed() {
        let html = parse_markdown("settings --> about \"phone\"");
        assert!(html.contains("settings --&gt; about \"phone\""));
        assert!(!html.contains('–'));
    }

    #[test]
    fn video_src_cannot_break_out() {
        let html = parse_markdown("![x](<a.mp4\"onload=\"alert(1)>)");
        assert!(!html.contains("\"onload"));
    }
}
