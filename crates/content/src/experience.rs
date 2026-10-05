use crate::project::{ArtConfig, Design};

#[derive(Debug, Clone, PartialEq)]
pub struct Experience {
    pub organization: &'static str,
    pub role: &'static str,
    pub period: &'static str,
    pub stack: &'static [&'static str],
    pub description: &'static str,
    pub art: ArtConfig,
}

pub static EXPERIENCES: &[Experience] = &[
    Experience {
        organization: "AEXLRT",
        role: "Software Engineering Intern",
        period: "Jun 2026 — Present",
        stack: &["Flutter", "Three.js", "Prisma", "CommonJS"],
        description: "Writing end-to-end, integration, and unit tests for game flows; fixing Flutter mobile issues and contributing Blender/Three.js landing-page assets and interactions.",
        art: ArtConfig {
            design: Design::Cube,
            seed: 611,
        },
    },
    Experience {
        organization: "Vedora",
        role: "Full-Stack Development Intern",
        period: "May 2026 — Jul 2026",
        stack: &[
            "React",
            "TypeScript",
            "Prisma",
            "PostgreSQL",
            "Redis",
            "Docker",
            "Azure",
        ],
        description: "Shipped authentication and session infrastructure, production migrations, transaction-safe repair tooling, CI safeguards, and AI tutoring workflows for a multi-tenant ed-tech platform.",
        art: ArtConfig {
            design: Design::Helix,
            seed: 2084,
        },
    },
    Experience {
        organization: "Virtual Waiter Platform",
        role: "Founding Engineer",
        period: "Jan 2026 — Present",
        stack: &["Next.js", "React", "Supabase", "SSE", "Web Push"],
        description: "Developing a multi-tenant real-time ordering platform with AI recommendations, tenant-aware routing, live order updates, push notifications, and load-tested checkout flows.",
        art: ArtConfig {
            design: Design::Stacks,
            seed: 4402,
        },
    },
];
