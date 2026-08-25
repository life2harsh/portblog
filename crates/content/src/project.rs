#[derive(Debug, Clone, PartialEq)]
pub struct ArtConfig {
    pub frequency: f64,
    pub amplitude: f64,
    pub twist: f64,
    pub noise: f64,
    pub count: u32,
    pub phase: f64,
    pub scale: f64,
    pub seed: u64,
}

impl ArtConfig {
    pub fn lerp(&self, target: &ArtConfig, t: f64) -> ArtConfig {
        let t = t.clamp(0.0, 1.0);
        ArtConfig {
            frequency: self.frequency + (target.frequency - self.frequency) * t,
            amplitude: self.amplitude + (target.amplitude - self.amplitude) * t,
            twist: self.twist + (target.twist - self.twist) * t,
            noise: self.noise + (target.noise - self.noise) * t,
            count: (self.count as f64 + (target.count as f64 - self.count as f64) * t).round()
                as u32,
            phase: self.phase + (target.phase - self.phase) * t,
            scale: self.scale + (target.scale - self.scale) * t,
            seed: if t < 0.5 { self.seed } else { target.seed },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectStatus {
    Active,
    Archived,
    Wip,
}

impl ProjectStatus {
    pub fn label(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::Archived => "archived",
            ProjectStatus::Wip => "wip",
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
    pub description: &'static str,
    pub long_description: &'static str,
    pub stack: &'static [&'static str],
    pub year: u16,
    pub status: ProjectStatus,
    pub links: &'static [ProjectLink],
    pub art: ArtConfig,
}

pub static PROJECTS: &[Project] = &[
    Project {
        slug: "portblog",
        name: "portblog",
        description: "This site. A Dioxus-based portfolio and blog with generative art and editorial design.",
        long_description: "A Rust/Dioxus workspace targeting WASM. Two apps share a common UI crate and design system. The portfolio features a two-column editorial layout with generative canvas art. The blog provides clean markdown rendering with keyboard-first navigation.",
        stack: &["Rust", "Dioxus 0.7", "WASM", "web-sys", "CSS"],
        year: 2026,
        status: ProjectStatus::Active,
        links: &[
            ProjectLink {
                label: "github",
                url: "https://github.com/life2harsh/portblog",
            },
            ProjectLink {
                label: "live",
                url: "https://life2harsh.xyz",
            },
        ],
        art: ArtConfig {
            frequency: 2.4,
            amplitude: 0.35,
            twist: 1.2,
            noise: 0.08,
            count: 48,
            phase: 0.0,
            scale: 1.0,
            seed: 1337,
        },
    },
    Project {
        slug: "helix-config",
        name: "helix-config",
        description: "A carefully maintained Helix editor configuration with custom themes and language server setup.",
        long_description: "A structured approach to editor configuration as code. Includes handcrafted color schemes respecting terminal palette constraints, tuned LSP settings for Rust, TypeScript, and Python, and keybinding layers that extend Helix's modal editing model.",
        stack: &["TOML", "Shell", "Nix"],
        year: 2025,
        status: ProjectStatus::Active,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/life2harsh/helix-config",
        }],
        art: ArtConfig {
            frequency: 1.1,
            amplitude: 0.55,
            twist: 0.3,
            noise: 0.22,
            count: 24,
            phase: 1.57,
            scale: 0.8,
            seed: 42,
        },
    },
    Project {
        slug: "noisetools",
        name: "noisetools",
        description: "A small Rust library for coherent noise — Perlin, Simplex, and domain-warped variants.",
        long_description: "Hand-rolled noise functions in pure Rust with no unsafe code. Provides gradient noise, value noise, and a domain-warp compositor. Designed for generative art and procedural generation, with WASM bindings and benchmark suite.",
        stack: &["Rust", "WASM", "wasm-bindgen"],
        year: 2025,
        status: ProjectStatus::Archived,
        links: &[
            ProjectLink {
                label: "github",
                url: "https://github.com/life2harsh/noisetools",
            },
            ProjectLink {
                label: "crates.io",
                url: "https://crates.io/crates/noisetools",
            },
        ],
        art: ArtConfig {
            frequency: 4.5,
            amplitude: 0.7,
            twist: 2.8,
            noise: 0.6,
            count: 80,
            phase: 0.78,
            scale: 1.4,
            seed: 7777,
        },
    },
    Project {
        slug: "schemr",
        name: "schemr",
        description: "Terminal color scheme generator that extracts palettes from reference images.",
        long_description: "A CLI tool that reads image files and generates terminal color schemes via k-means clustering and perceptual color space conversion. Outputs formats for Alacritty, Kitty, and Wezterm. Explores the gap between visual design tools and developer environments.",
        stack: &["Rust", "CLI", "image processing"],
        year: 2024,
        status: ProjectStatus::Wip,
        links: &[ProjectLink {
            label: "github",
            url: "https://github.com/life2harsh/schemr",
        }],
        art: ArtConfig {
            frequency: 0.8,
            amplitude: 0.2,
            twist: 0.05,
            noise: 0.02,
            count: 16,
            phase: std::f64::consts::PI,
            scale: 2.0,
            seed: 999,
        },
    },
];

pub fn find_project(slug: &str) -> Option<&'static Project> {
    PROJECTS.iter().find(|p| p.slug == slug)
}
