#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Design {
    Burst,
    Moth,
    Worm,
    Carousel,
    Tower,
    Invader,
    Ribbon,
    Fan,
    Orbit,
    Domino,
    Tiles,
    Cube,
    Helix,
    Stacks,
    Agenda,
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Cpp,
    C,
    Java,
    Dart,
    Fork,
    Mesh,
    Knot,
}

impl Design {
    pub const ALL: [Design; 14] = [
        Design::Burst,
        Design::Moth,
        Design::Worm,
        Design::Carousel,
        Design::Tower,
        Design::Invader,
        Design::Ribbon,
        Design::Fan,
        Design::Orbit,
        Design::Domino,
        Design::Tiles,
        Design::Cube,
        Design::Helix,
        Design::Stacks,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtConfig {
    pub design: Design,
    pub seed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectStatus {
    Active,
    InProgress,
}

impl ProjectStatus {
    pub fn label(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::InProgress => "in progress",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectLink {
    pub label: &'static str,
    pub url: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub slug: &'static str,
    pub name: &'static str,
    pub year: &'static str,
    pub description: &'static str,
    pub stack: &'static [&'static str],
    pub status: ProjectStatus,
    pub links: &'static [ProjectLink],
    pub long_description: Option<&'static str>,
    pub live_url: Option<&'static str>,
    pub previews: &'static [&'static str],
    pub art: ArtConfig,
}

pub static PROJECTS: &[Project] = &[
    Project {
        slug: "wimg",
        name: "WIMG",
        year: "2025",
        description: "Terminal media viewer for images, GIFs and video with adaptive terminal rendering and frame-aware playback.",
        stack: &["Rust", "FFmpeg", "Sixel", "image-rs"],
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/life2harsh/wimg",
        }],
        long_description: None,
        live_url: None,
        previews: &["/previews/wimg.webp"],
        art: ArtConfig {
            design: Design::Carousel,
            seed: 1482,
        },
    },
    Project {
        slug: "stonepen",
        name: "Stonepen",
        year: "2026",
        description: "Editable vector handwriting engine for pen, touch and mouse input with vector strokes, geometry, undo/redo and export.",
        stack: &["Rust", "WebAssembly", "wasm-bindgen"],
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/life2harsh/stonepen",
        }],
        long_description: None,
        live_url: None,
        previews: &["/previews/stonepen.webp"],
        art: ArtConfig {
            design: Design::Worm,
            seed: 723,
        },
    },
    Project {
        slug: "ytuff",
        name: "YTuff",
        year: "2026",
        description: "Terminal music player for local audio and YouTube Music with playlists, lyrics, downloads, artwork, background playback and OS media integration.",
        stack: &["Rust", "Ratatui", "FFmpeg", "Rodio", "Symphonia"],
        status: ProjectStatus::InProgress,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/life2harsh/ytuff",
        }],
        long_description: None,
        live_url: None,
        previews: &["/previews/ytuff.webp"],
        art: ArtConfig {
            design: Design::Orbit,
            seed: 3311,
        },
    },
    Project {
        slug: "whatagent",
        name: "whatagent",
        year: "2026",
        description: "Text yourself on WhatsApp, or send a voice note, and it lands in Google Calendar or your todo list, with reminders back in the same chat.",
        stack: &["Rust", "Tokio", "Gemini", "SQLite", "Google Calendar API"],
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/life2harsh/whatagent",
        }],
        long_description: None,
        live_url: None,
        previews: &["/previews/whatagent.webp"],
        art: ArtConfig {
            design: Design::Agenda,
            seed: 6150,
        },
    },
    Project {
        slug: "echopath",
        name: "EchoPath",
        year: "2026",
        description: "Audio analysis and playlist recommendation using signal features, ML models, and search and recommendation algorithms.",
        stack: &["Python", "PyTorch", "librosa", "scikit-learn", "FastAPI"],
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/life2harsh/EchoPath",
        }],
        long_description: None,
        live_url: None,
        previews: &[],
        art: ArtConfig {
            design: Design::Tiles,
            seed: 9017,
        },
    },
    Project {
        slug: "git-orientation",
        name: "Git Orientation",
        year: "2025",
        description: "Puzzle-style Git and GitHub orientation for OSDC: solve questions in a CRT-styled web terminal, submit answers by pull request, and let CI check them.",
        stack: &["JavaScript", "WebGL", "GitHub Actions"],
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/osdc/git-orientation",
        }],
        long_description: None,
        live_url: None,
        previews: &["/previews/git-orientation.webp"],
        art: ArtConfig {
            design: Design::Domino,
            seed: 5150,
        },
    },
    Project {
        slug: "cocoon",
        name: "Cocoon",
        year: "2026",
        description: "Private journaling app with a React frontend, a FastAPI backend, Google sign-in and installable reminders.",
        stack: &["TypeScript", "React", "FastAPI", "SQLite"],
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/life2harsh/cocoon",
        }],
        long_description: None,
        live_url: None,
        previews: &[
            "/previews/cocoon-1.webp",
            "/previews/cocoon-2.webp",
            "/previews/cocoon-3.webp",
            "/previews/cocoon-4.webp",
        ],
        art: ArtConfig {
            design: Design::Moth,
            seed: 2718,
        },
    },
    Project {
        slug: "repobrowser",
        name: "RepoBrowser",
        year: "2025",
        description: "Learn your way around a repository through AI annotations and summaries of its files.",
        stack: &["TypeScript", "Next.js", "React"],
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/life2harsh/RepoBrowser",
        }],
        long_description: None,
        live_url: Some("https://repobrowser.vercel.app"),
        previews: &["/previews/repobrowser.webp"],
        art: ArtConfig {
            design: Design::Fan,
            seed: 6604,
        },
    },
    Project {
        slug: "osdweb",
        name: "OSDWeb",
        year: "2026",
        description: "The new official OSDC website, a ground-up revamp of osdc.dev.",
        stack: &["TypeScript", "Next.js", "Three.js", "Turborepo"],
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/osdc/osdweb",
        }],
        long_description: None,
        live_url: None,
        previews: &["/previews/osdweb.webp", "/previews/osdweb-2.webp"],
        art: ArtConfig {
            design: Design::Tower,
            seed: 4096,
        },
    },
    Project {
        slug: "osdhack26",
        name: "OSDHack26",
        year: "2026",
        description: "Site for OSDHack 26, with registration, the rulebook and a multiplayer arcade of browser games.",
        stack: &["JavaScript", "Python", "Canvas"],
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/osdc/osdhack26",
        }],
        long_description: None,
        live_url: None,
        previews: &["/previews/osdhack26.webp", "/previews/osdhack26-2.webp"],
        art: ArtConfig {
            design: Design::Invader,
            seed: 2626,
        },
    },
    Project {
        slug: "ricing",
        name: "Ricing",
        year: "2026",
        description: "Agentic Linux ricing assistant that inspects your setup, plans changes, installs packages and patches configs.",
        stack: &["Go", "JavaScript", "Hyprland"],
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/wowtuff/ricing",
        }],
        long_description: None,
        live_url: Some("https://wowtuff.github.io/ricing/"),
        previews: &["/previews/ricing.webp"],
        art: ArtConfig {
            design: Design::Ribbon,
            seed: 7741,
        },
    },
];

pub fn find_project(slug: &str) -> Option<&'static Project> {
    PROJECTS.iter().find(|project| project.slug == slug)
}
