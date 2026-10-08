use crate::experience::{EXPERIENCES, Experience};
use crate::project::{ArtConfig, Design, PROJECTS, Project, ProjectLink};

#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: &'static str,
    pub tagline: &'static str,
    pub frameworks: &'static [&'static str],
    pub matches: &'static [&'static str],
    pub repos: &'static [ProjectLink],
    pub description: &'static str,
    pub art: ArtConfig,
}

impl Tool {
    fn used(&self, stack: &[&str]) -> bool {
        stack.iter().any(|item| {
            self.matches
                .iter()
                .any(|name| item.eq_ignore_ascii_case(name))
        })
    }

    pub fn projects(&self) -> impl Iterator<Item = (usize, &'static Project)> + '_ {
        PROJECTS
            .iter()
            .enumerate()
            .filter(|(_, project)| self.used(project.stack))
    }

    pub fn experiences(&self) -> impl Iterator<Item = (usize, &'static Experience)> + '_ {
        EXPERIENCES
            .iter()
            .enumerate()
            .filter(|(_, item)| self.used(item.stack))
    }

    pub fn count(&self) -> usize {
        self.projects().count() + self.experiences().count() + self.repos.len()
    }
}

const fn repo(label: &'static str, url: &'static str) -> ProjectLink {
    ProjectLink { label, url }
}

const fn art(design: Design, seed: u64) -> ArtConfig {
    ArtConfig { design, seed }
}

pub static STACK: &[Tool] = &[
    Tool {
        name: "Rust",
        tagline: "systems, wasm, terminal apps",
        frameworks: &[
            "Dioxus",
            "Ratatui",
            "Axum",
            "Tokio",
            "wasm-bindgen",
            "russh",
            "sqlx",
            "Rodio",
            "Symphonia",
        ],
        matches: &["Rust"],
        repos: &[
            repo("portblog", "https://github.com/life2harsh/portblog"),
            repo("inkstone", "https://github.com/life2harsh/inkstone"),
        ],
        description: "The language I reach for first. This site, its ssh twin and the blog api are all Rust, and so are my terminal media tools and the handwriting engine.",
        art: art(Design::Rust, 1015),
    },
    Tool {
        name: "TypeScript",
        tagline: "web apps, frontends",
        frameworks: &[
            "React",
            "Next.js",
            "Vite",
            "TanStack Query",
            "Zustand",
            "Tailwind CSS",
            "Express",
            "Prisma",
            "Zod",
            "Three.js",
            "Turborepo",
        ],
        matches: &["TypeScript"],
        repos: &[
            repo("gigflow", "https://github.com/life2harsh/GigFlow"),
            repo("battleroom", "https://github.com/life2harsh/battleroom"),
            repo("codehear", "https://github.com/life2harsh/codehear"),
        ],
        description: "Most of my web work. React and Next.js frontends, Express and Prisma backends, and a VS Code extension built on tree-sitter.",
        art: art(Design::TypeScript, 2203),
    },
    Tool {
        name: "Python",
        tagline: "backends, ml, data",
        frameworks: &[
            "FastAPI",
            "Django",
            "Flask",
            "PyTorch",
            "scikit-learn",
            "librosa",
            "pandas",
            "NumPy",
            "Transformers",
            "XGBoost",
            "Textual",
        ],
        matches: &["Python", "FastAPI", "Django", "Flask"],
        repos: &[
            repo("whatbytes", "https://github.com/life2harsh/whatbytes"),
            repo("pollhack", "https://github.com/life2harsh/pollhack"),
            repo("adobe1b", "https://github.com/life2harsh/Adobe1B"),
        ],
        description: "Backends and models. FastAPI and Django services, audio and ranking models in PyTorch and scikit-learn, and the odd terminal app in Textual.",
        art: art(Design::Python, 3307),
    },
    Tool {
        name: "JavaScript",
        tagline: "the browser, canvas, webgl",
        frameworks: &[
            "Node.js", "Express", "WebGL", "Canvas", "GSAP", "Chart.js", "Leaflet", "jQuery",
        ],
        matches: &["JavaScript", "CommonJS"],
        repos: &[
            repo("turingsim", "https://github.com/life2harsh/turingsim"),
            repo(
                "laserbeamdeviation",
                "https://github.com/life2harsh/laserbeamdeviation",
            ),
        ],
        description: "Where I started on the web. Canvas and WebGL experiments, browser games for OSDHack, simulators and plenty of small tools.",
        art: art(Design::JavaScript, 4409),
    },
    Tool {
        name: "Go",
        tagline: "tooling, desktop",
        frameworks: &["Fyne", "gorilla/websocket"],
        matches: &["Go"],
        repos: &[],
        description: "The ricing assistant's server and its desktop app, written with Fyne and websockets.",
        art: art(Design::Go, 5501),
    },
    Tool {
        name: "C++",
        tagline: "games, gui, scraping",
        frameworks: &["Qt", "SFML", "libcurl", "libxml2"],
        matches: &["C++"],
        repos: &[
            repo("spaceshooter", "https://github.com/life2harsh/SpaceShooter"),
            repo("gamelibrary", "https://github.com/life2harsh/GameLibrary"),
            repo(
                "gsoc-scrapper",
                "https://github.com/life2harsh/gsoc-scrapper",
            ),
        ],
        description: "A space shooter in SFML, a Qt game library app and a GSoC organisation scraper on libcurl and libxml2.",
        art: art(Design::Cpp, 6607),
    },
    Tool {
        name: "C",
        tagline: "unix, terminals",
        frameworks: &["ncurses", "POSIX"],
        matches: &["C"],
        repos: &[repo("unixpbl", "https://github.com/life2harsh/unixpbl")],
        description: "An ncurses program on raw POSIX: directories, users, processes and signals.",
        art: art(Design::C, 7703),
    },
    Tool {
        name: "Java",
        tagline: "android, spring",
        frameworks: &["Spring Boot", "Android"],
        matches: &["Java"],
        repos: &[
            repo(
                "wifiautologger",
                "https://github.com/life2harsh/WiFiAutologger",
            ),
            repo(
                "autowifiloginscript",
                "https://github.com/life2harsh/AutoWifiLoginScript",
            ),
        ],
        description: "A Wi-Fi autologin script that grew into a Spring Boot backend, plus some Android.",
        art: art(Design::Java, 8807),
    },
    Tool {
        name: "Dart",
        tagline: "flutter, mobile",
        frameworks: &["Flutter"],
        matches: &["Dart", "Flutter"],
        repos: &[repo(
            "wifiautologger",
            "https://github.com/life2harsh/WiFiAutologger",
        )],
        description: "Flutter apps: the Wi-Fi autologger's client, and mobile fixes at AEXLRT.",
        art: art(Design::Dart, 9901),
    },
];
