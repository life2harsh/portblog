use art::Field;
use content::{ArtConfig, Design, EXPERIENCES, Entry, PROJECTS, STACK};
use ratatui::style::Color;

use crate::journal::{Shared, Status};
use crate::keys::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    About,
    Projects,
    Experience,
    Stack,
    Writing,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::About,
        Tab::Projects,
        Tab::Experience,
        Tab::Stack,
        Tab::Writing,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tab::About => "about",
            Tab::Projects => "projects",
            Tab::Experience => "experience",
            Tab::Stack => "stack",
            Tab::Writing => "writing",
        }
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0)
    }
}

pub const SOCIALS: [(&str, &str, Design); 3] = [
    ("GITHUB", "github.com/life2harsh", Design::Fork),
    ("LINKEDIN", "linkedin.com/in/life2harsh", Design::Mesh),
    ("EMAIL", "jhaharsh29@gmail.com", Design::Knot),
];

pub const THEMES: [(&str, Color); 5] = [
    ("ACID", Color::Rgb(212, 245, 60)),
    ("ICE", Color::Rgb(127, 230, 247)),
    ("COBALT", Color::Rgb(95, 130, 255)),
    ("BURNT", Color::Rgb(255, 154, 74)),
    ("WARM", Color::Rgb(255, 106, 95)),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Quit,
}

pub struct App {
    pub tab: Tab,
    pub project: usize,
    pub experience: usize,
    pub tool: usize,
    pub social: Option<usize>,
    pub entry: usize,
    pub reading: bool,
    pub scroll: u16,
    pub theme: usize,
    pub help: bool,
    pub field: Field,
    pub entries: Vec<Entry>,
    pub status: Status,
    journal: Shared,
    journal_version: u64,
    since_draw: f64,
    dirty: bool,
}

impl App {
    pub fn new(journal: Shared) -> Self {
        let mut app = Self {
            tab: Tab::About,
            project: 0,
            experience: 0,
            tool: 0,
            social: None,
            entry: 0,
            reading: false,
            scroll: 0,
            theme: 2,
            help: false,
            field: Field::with_resolution(about_art(), 52, 24),
            entries: Vec::new(),
            status: Status::Loading,
            journal,
            journal_version: u64::MAX,
            since_draw: 0.0,
            dirty: true,
        };
        app.sync_journal();
        app
    }

    pub fn accent(&self) -> Color {
        THEMES[self.theme].1
    }

    pub fn theme_label(&self) -> &'static str {
        THEMES[self.theme].0
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.entries.get(self.entry)
    }

    pub fn handle(&mut self, key: Key) -> Outcome {
        self.dirty = true;
        if self.help {
            if matches!(key, Key::Interrupt | Key::Char('q')) {
                return Outcome::Quit;
            }
            self.help = false;
            return Outcome::Continue;
        }
        if let Some(index) = self.social {
            let delta = match key {
                Key::Down | Key::Right | Key::Tab | Key::Char('j' | 'l') => Some(1),
                Key::Up | Key::Left | Key::BackTab | Key::Char('k' | 'h') => Some(-1),
                _ => None,
            };
            if let Some(delta) = delta {
                let next = (index as i64 + delta).rem_euclid(SOCIALS.len() as i64) as usize;
                self.social = Some(next);
                self.field.transition_to(self.art_target());
                return Outcome::Continue;
            }
            match key {
                Key::Escape | Key::Backspace | Key::Char('c') => {
                    self.social = None;
                    self.field.transition_to(self.art_target());
                    return Outcome::Continue;
                }
                Key::Enter => return Outcome::Continue,
                Key::Interrupt | Key::Char('q' | '?' | 't') => {}
                _ => self.social = None,
            }
        }
        match key {
            Key::Interrupt | Key::Char('q') => return Outcome::Quit,
            Key::Char('?') => self.help = true,
            Key::Char('c') if !self.reading => self.social = Some(0),
            Key::Char('1' | 'a') => self.switch(Tab::About),
            Key::Char('2' | 'p') => self.switch(Tab::Projects),
            Key::Char('3' | 'e') => self.switch(Tab::Experience),
            Key::Char('4' | 's') => self.switch(Tab::Stack),
            Key::Char('5' | 'w') => self.switch(Tab::Writing),
            Key::Tab | Key::Right | Key::Char('l') if !self.reading => {
                self.switch(Tab::ALL[(self.tab.index() + 1) % Tab::ALL.len()])
            }
            Key::BackTab | Key::Left | Key::Char('h') if !self.reading => {
                self.switch(Tab::ALL[(self.tab.index() + Tab::ALL.len() - 1) % Tab::ALL.len()])
            }
            Key::Char('t') => self.theme = (self.theme + 1) % THEMES.len(),
            Key::Down | Key::Char('j') => self.step(1),
            Key::Up | Key::Char('k') => self.step(-1),
            Key::PageDown | Key::Char(' ') if self.reading => {
                self.scroll = self.scroll.saturating_add(10)
            }
            Key::PageUp if self.reading => self.scroll = self.scroll.saturating_sub(10),
            Key::Home | Key::Char('g') => self.jump(false),
            Key::End | Key::Char('G') => self.jump(true),
            Key::Enter if self.tab == Tab::About => self.switch(Tab::Experience),
            Key::Enter if self.tab == Tab::Writing && !self.entries.is_empty() => {
                self.reading = true;
                self.scroll = 0;
            }
            Key::Escape | Key::Backspace if self.reading => self.reading = false,
            _ => self.dirty = false,
        }
        self.field.transition_to(self.art_target());
        Outcome::Continue
    }

    fn switch(&mut self, tab: Tab) {
        self.tab = tab;
        self.reading = false;
    }

    fn step(&mut self, delta: i64) {
        if self.reading {
            self.scroll = if delta > 0 {
                self.scroll.saturating_add(1)
            } else {
                self.scroll.saturating_sub(1)
            };
            return;
        }
        let (index, len) = match self.tab {
            Tab::About | Tab::Experience => (&mut self.experience, EXPERIENCES.len()),
            Tab::Projects => (&mut self.project, PROJECTS.len()),
            Tab::Stack => (&mut self.tool, STACK.len()),
            Tab::Writing => (&mut self.entry, self.entries.len()),
        };
        if len > 0 {
            *index = (*index as i64 + delta).rem_euclid(len as i64) as usize;
        }
    }

    fn jump(&mut self, end: bool) {
        if self.reading {
            self.scroll = if end { u16::MAX / 2 } else { 0 };
            return;
        }
        let (index, len) = match self.tab {
            Tab::About | Tab::Experience => (&mut self.experience, EXPERIENCES.len()),
            Tab::Projects => (&mut self.project, PROJECTS.len()),
            Tab::Stack => (&mut self.tool, STACK.len()),
            Tab::Writing => (&mut self.entry, self.entries.len()),
        };
        *index = if end { len.saturating_sub(1) } else { 0 };
    }

    fn art_target(&self) -> ArtConfig {
        if let Some(index) = self.social {
            return ArtConfig {
                design: SOCIALS[index].2,
                seed: 31 + index as u64,
            };
        }
        match self.tab {
            Tab::About => about_art(),
            Tab::Projects => PROJECTS[self.project].art,
            Tab::Experience => EXPERIENCES[self.experience].art,
            Tab::Stack => STACK[self.tool].art,
            Tab::Writing => self
                .selected_entry()
                .map_or_else(about_art, |entry| entry_art(entry.seed())),
        }
    }

    fn sync_journal(&mut self) {
        let snapshot = self
            .journal
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if snapshot.version == self.journal_version {
            return;
        }
        self.journal_version = snapshot.version;
        self.status = snapshot.status;
        let selected = self.selected_entry().map(|entry| entry.slug.clone());
        self.entries = snapshot.entries.clone();
        drop(snapshot);
        self.entry = selected
            .and_then(|slug| self.entries.iter().position(|entry| entry.slug == slug))
            .unwrap_or(0);
        if self.entries.is_empty() {
            self.reading = false;
        }
        self.dirty = true;
        self.field.transition_to(self.art_target());
    }

    pub fn tick(&mut self, dt: f64) -> bool {
        self.sync_journal();
        self.field.step(dt, true);
        self.since_draw += dt;
        let due = self.dirty
            || !self.field.settled() && self.since_draw >= 0.08
            || self.since_draw >= 0.5;
        if due {
            self.dirty = false;
            self.since_draw = 0.0;
        }
        due
    }
}

pub fn about_art() -> ArtConfig {
    ArtConfig {
        design: Design::Burst,
        seed: 84921,
    }
}

fn entry_art(seed: u64) -> ArtConfig {
    ArtConfig {
        design: Design::ALL[(seed % Design::ALL.len() as u64) as usize],
        seed: seed % 99_991,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal;

    fn app() -> App {
        App::new(journal::shared())
    }

    #[test]
    fn tabs_and_wrapping() {
        let mut app = app();
        assert_eq!(app.tab, Tab::About);
        app.handle(Key::Char('2'));
        assert_eq!(app.tab, Tab::Projects);
        app.handle(Key::Up);
        assert_eq!(app.project, PROJECTS.len() - 1);
        app.handle(Key::Char('j'));
        assert_eq!(app.project, 0);
        app.handle(Key::Tab);
        assert_eq!(app.tab, Tab::Experience);
        app.handle(Key::BackTab);
        assert_eq!(app.tab, Tab::Projects);
        app.handle(Key::Char('s'));
        app.handle(Key::Up);
        assert_eq!(app.tool, STACK.len() - 1);
        app.handle(Key::Tab);
        assert_eq!(app.tab, Tab::Writing);
        app.handle(Key::Char('a'));
        app.handle(Key::Enter);
        assert_eq!(app.tab, Tab::Experience);
        app.handle(Key::Char('p'));
        app.handle(Key::Char('t'));
        assert_eq!(app.theme_label(), "BURNT");
    }

    #[test]
    fn connect_is_a_section() {
        let mut app = app();
        app.handle(Key::Char('c'));
        assert_eq!(app.social, Some(0));
        app.handle(Key::Down);
        app.handle(Key::Char('l'));
        assert_eq!(app.social, Some(2));
        app.handle(Key::Right);
        assert_eq!(app.social, Some(0));
        app.handle(Key::Up);
        assert_eq!(app.social, Some(2));
        app.handle(Key::Char('t'));
        assert_eq!(app.social, Some(2));
        assert_eq!(app.experience, 0);
        app.handle(Key::Escape);
        assert_eq!(app.social, None);
        app.handle(Key::Char('c'));
        app.handle(Key::Char('s'));
        assert_eq!(app.social, None);
        assert_eq!(app.tab, Tab::Stack);
    }

    #[test]
    fn quitting_and_help() {
        let mut app = app();
        app.handle(Key::Char('?'));
        assert!(app.help);
        assert_eq!(app.handle(Key::Char('x')), Outcome::Continue);
        assert!(!app.help);
        assert_eq!(app.handle(Key::Char('q')), Outcome::Quit);
        assert_eq!(app.handle(Key::Interrupt), Outcome::Quit);
    }

    #[test]
    fn reading_needs_entries() {
        let shared = journal::shared();
        let mut app = App::new(shared.clone());
        app.handle(Key::Char('5'));
        app.handle(Key::Enter);
        assert!(!app.reading);
        {
            let mut snapshot = shared.write().unwrap();
            snapshot.version = 1;
            snapshot.status = Status::Ready;
            snapshot.entries = vec![Entry {
                slug: "hello".into(),
                kind: content::EntryKind::Note,
                title: None,
                summary: None,
                body: "hello world".into(),
                tags: vec![],
                published: true,
                created_at: 0,
                updated_at: 0,
            }];
        }
        app.tick(0.016);
        app.handle(Key::Enter);
        assert!(app.reading);
        app.handle(Key::Char('j'));
        assert_eq!(app.scroll, 1);
        app.handle(Key::Escape);
        assert!(!app.reading);
    }
}
