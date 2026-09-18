use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    #[default]
    Note,
    Essay,
}

impl EntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Note => "note",
            Self::Essay => "essay",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "note" => Some(Self::Note),
            "essay" => Some(Self::Essay),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub slug: String,
    pub kind: EntryKind,
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    pub body: String,
    pub tags: Vec<String>,
    pub published: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntryDraft {
    pub kind: Option<EntryKind>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub body: Option<String>,
    pub tags: Option<Vec<String>>,
    pub published: Option<bool>,
}

impl Entry {
    pub fn heading(&self) -> String {
        match self.title.as_deref().map(str::trim) {
            Some(title) if !title.is_empty() => title.to_string(),
            _ => first_words(&self.body, 9),
        }
    }

    pub fn description(&self, chars: usize) -> String {
        match self.summary.as_deref().map(str::trim) {
            Some(summary) if !summary.is_empty() => summary.to_string(),
            _ => self.excerpt(chars),
        }
    }

    pub fn excerpt(&self, chars: usize) -> String {
        let plain = self
            .body
            .lines()
            .map(|line| line.trim_start_matches(['#', '>', '-', '*', ' ']))
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if plain.chars().count() <= chars {
            plain
        } else {
            let cut: String = plain.chars().take(chars + 1).collect();
            let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
            format!("{}…", cut.trim_end_matches([' ', ',', '.', ';', ':']))
        }
    }

    pub fn reading_minutes(&self) -> u32 {
        (self.body.split_whitespace().count() as u32)
            .div_ceil(220)
            .max(1)
    }

    pub fn seed(&self) -> u64 {
        self.slug
            .bytes()
            .fold(1469598103934665603u64, |hash, byte| {
                (hash ^ byte as u64).wrapping_mul(1099511628211)
            })
    }
}

fn first_words(text: &str, count: usize) -> String {
    let words = text
        .split_whitespace()
        .map(|word| word.trim_matches(['#', '*', '_', '>', '`']))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let mut heading = words
        .iter()
        .take(count)
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    if words.len() > count {
        heading.push('…');
    }
    heading
}

pub fn slugify(text: &str) -> String {
    let mut slug = String::new();
    for character in text.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
        if slug.len() >= 60 {
            break;
        }
    }
    slug.trim_end_matches('-').to_string()
}

pub fn format_date(timestamp: i64) -> String {
    let days = timestamp.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(format_date(0), "1970-01-01");
        assert_eq!(format_date(1_790_899_200), "2026-10-02");
        assert_eq!(format_date(951_782_400), "2000-02-29");
    }

    #[test]
    fn slugs() {
        assert_eq!(
            slugify("Hello, World! Rust & WASM"),
            "hello-world-rust-wasm"
        );
        assert_eq!(slugify("  --  "), "");
    }

    #[test]
    fn headings() {
        let mut entry = Entry {
            slug: "a".into(),
            kind: EntryKind::Note,
            title: None,
            summary: None,
            body: "# one two three four five six seven eight nine ten".into(),
            tags: vec![],
            published: true,
            created_at: 0,
            updated_at: 0,
        };
        assert_eq!(
            entry.heading(),
            "one two three four five six seven eight nine…"
        );
        entry.title = Some("Titled".into());
        assert_eq!(entry.heading(), "Titled");
        assert_eq!(entry.reading_minutes(), 1);
        assert_eq!(entry.description(20), "one two three four…");
        entry.summary = Some("why rings".into());
        assert_eq!(entry.description(20), "why rings");
    }
}
