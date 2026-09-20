use content::{Entry, EntryDraft, EntryKind, slugify};
use rusqlite::{Connection, OptionalExtension, Row, params};

pub struct Store {
    connection: Connection,
}

#[derive(Debug)]
pub enum StoreError {
    NotFound,
    Invalid(&'static str),
    Database(rusqlite::Error),
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

const COLUMNS: &str = "slug, kind, title, body, tags, published, created_at, updated_at, summary";

impl Store {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS entries (
                slug TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                title TEXT,
                body TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                published INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS entries_created ON entries (created_at DESC);",
        )?;
        let has_summary = connection
            .prepare("SELECT 1 FROM pragma_table_info('entries') WHERE name = 'summary'")?
            .exists([])?;
        if !has_summary {
            connection.execute("ALTER TABLE entries ADD COLUMN summary TEXT", [])?;
        }
        Ok(Self { connection })
    }

    pub fn list(
        &self,
        include_drafts: bool,
        kind: Option<EntryKind>,
    ) -> Result<Vec<Entry>, StoreError> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT {COLUMNS} FROM entries
             WHERE (?1 OR published = 1) AND (?2 IS NULL OR kind = ?2)
             ORDER BY created_at DESC, slug DESC"
        ))?;
        let rows = statement.query_map(
            params![include_drafts, kind.map(EntryKind::as_str)],
            read_entry,
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get(&self, slug: &str, include_drafts: bool) -> Result<Entry, StoreError> {
        self.connection
            .query_row(
                &format!("SELECT {COLUMNS} FROM entries WHERE slug = ?1 AND (?2 OR published = 1)"),
                params![slug, include_drafts],
                read_entry,
            )
            .optional()?
            .ok_or(StoreError::NotFound)
    }

    pub fn create(&self, draft: EntryDraft, now: i64) -> Result<Entry, StoreError> {
        let body = draft.body.unwrap_or_default();
        if body.trim().is_empty() {
            return Err(StoreError::Invalid("body is empty"));
        }
        let kind = draft.kind.unwrap_or_default();
        let title = clean_title(draft.title);
        let base = match &title {
            Some(title) => slugify(title),
            None => slugify(
                &body
                    .split_whitespace()
                    .take(6)
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
        };
        let base = if base.is_empty() {
            format!("entry-{now}")
        } else {
            base
        };
        let slug = self.unique_slug(&base)?;
        let entry = Entry {
            slug,
            kind,
            title,
            summary: clean_summary(draft.summary),
            body,
            tags: clean_tags(draft.tags.unwrap_or_default()),
            published: draft.published.unwrap_or(true),
            created_at: now,
            updated_at: now,
        };
        self.connection.execute(
            &format!("INSERT INTO entries ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"),
            params![
                entry.slug,
                entry.kind.as_str(),
                entry.title,
                entry.body,
                serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".into()),
                entry.published,
                entry.created_at,
                entry.updated_at,
                entry.summary
            ],
        )?;
        Ok(entry)
    }

    pub fn update(&self, slug: &str, draft: EntryDraft, now: i64) -> Result<Entry, StoreError> {
        let mut entry = self.get(slug, true)?;
        if let Some(kind) = draft.kind {
            entry.kind = kind;
        }
        if draft.title.is_some() {
            entry.title = clean_title(draft.title);
        }
        if draft.summary.is_some() {
            entry.summary = clean_summary(draft.summary);
        }
        if let Some(body) = draft.body {
            if body.trim().is_empty() {
                return Err(StoreError::Invalid("body is empty"));
            }
            entry.body = body;
        }
        if let Some(tags) = draft.tags {
            entry.tags = clean_tags(tags);
        }
        if let Some(published) = draft.published {
            entry.published = published;
        }
        entry.updated_at = now;
        self.connection.execute(
            "UPDATE entries SET kind = ?2, title = ?3, body = ?4, tags = ?5, published = ?6, updated_at = ?7,
             summary = ?8 WHERE slug = ?1",
            params![
                entry.slug,
                entry.kind.as_str(),
                entry.title,
                entry.body,
                serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".into()),
                entry.published,
                entry.updated_at,
                entry.summary
            ],
        )?;
        Ok(entry)
    }

    pub fn delete(&self, slug: &str) -> Result<(), StoreError> {
        match self
            .connection
            .execute("DELETE FROM entries WHERE slug = ?1", params![slug])?
        {
            0 => Err(StoreError::NotFound),
            _ => Ok(()),
        }
    }

    fn unique_slug(&self, base: &str) -> Result<String, StoreError> {
        let mut candidate = base.to_string();
        let mut suffix = 2;
        while self
            .connection
            .query_row(
                "SELECT 1 FROM entries WHERE slug = ?1",
                params![candidate],
                |_| Ok(()),
            )
            .optional()?
            .is_some()
        {
            candidate = format!("{base}-{suffix}");
            suffix += 1;
        }
        Ok(candidate)
    }
}

fn read_entry(row: &Row<'_>) -> rusqlite::Result<Entry> {
    let kind: String = row.get(1)?;
    let tags: String = row.get(4)?;
    Ok(Entry {
        slug: row.get(0)?,
        kind: EntryKind::parse(&kind).unwrap_or_default(),
        title: row.get(2)?,
        body: row.get(3)?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        published: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        summary: row.get(8)?,
    })
}

fn clean_title(title: Option<String>) -> Option<String> {
    title
        .map(|title| title.trim().chars().take(140).collect::<String>())
        .filter(|title| !title.is_empty())
}

fn clean_summary(summary: Option<String>) -> Option<String> {
    summary
        .map(|summary| summary.trim().chars().take(280).collect::<String>())
        .filter(|summary| !summary.is_empty())
}

fn clean_tags(tags: Vec<String>) -> Vec<String> {
    let mut cleaned = Vec::new();
    for tag in tags {
        let tag = slugify(&tag);
        if !tag.is_empty() && !cleaned.contains(&tag) && cleaned.len() < 8 {
            cleaned.push(tag);
        }
    }
    cleaned
}
