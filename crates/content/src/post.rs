#[derive(Debug, Clone, PartialEq)]
pub struct PostMeta {
    pub slug: &'static str,
    pub title: &'static str,
    pub date: &'static str,
    pub tags: &'static [&'static str],
    pub read_time_min: u8,
    pub excerpt: &'static str,
}

#[derive(Debug, Clone)]
pub struct Post {
    pub meta: PostMeta,
    pub body: &'static str,
}

pub static POSTS: &[Post] = &[
    Post {
        meta: PostMeta {
            slug: "writing-a-custom-allocator",
            title: "Writing a Custom Allocator in Rust",
            date: "2026-08-28",
            tags: &["rust", "systems", "memory"],
            read_time_min: 8,
            excerpt: "A walkthrough of implementing a bump allocator and why you might want to avoid the global allocator in embedded or WASM contexts.",
        },
        body: include_str!("../../../content/posts/writing-a-custom-allocator.md"),
    },
    Post {
        meta: PostMeta {
            slug: "on-deliberate-design",
            title: "On Deliberate Design",
            date: "2026-08-15",
            tags: &["design", "craft", "tools"],
            read_time_min: 5,
            excerpt: "Why most developer tools look the way they do, and what it takes to care about the other 5%.",
        },
        body: include_str!("../../../content/posts/on-deliberate-design.md"),
    },
    Post {
        meta: PostMeta {
            slug: "terminal-aesthetics",
            title: "Terminal Aesthetics: A Field Guide",
            date: "2026-07-30",
            tags: &["tui", "design", "tooling"],
            read_time_min: 6,
            excerpt: "Documenting the quiet beauty of well-crafted terminal interfaces — from UNIX pipes to Ratatui.",
        },
        body: include_str!("../../../content/posts/terminal-aesthetics.md"),
    },
];

pub fn find_post(slug: &str) -> Option<&'static Post> {
    POSTS.iter().find(|p| p.meta.slug == slug)
}
