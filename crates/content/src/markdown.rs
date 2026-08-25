use pulldown_cmark::{Options, Parser, html};

pub fn parse_markdown(src: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(src, options);
    let mut html_output = String::with_capacity(src.len() * 2);
    html::push_html(&mut html_output, parser);
    html_output
}
