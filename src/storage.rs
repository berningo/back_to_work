use std::{collections::HashMap, path::PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::model::Task;

type StoredTask = (i64, i64, String, bool);
type LoadedState = (Vec<Task>, Option<u64>, u64);

pub(crate) fn database_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_DATA_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.join("back_to_work").join("tasks.sqlite3")
}

pub(crate) fn open(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS tasks (
             id INTEGER PRIMARY KEY,
             parent_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE,
             position INTEGER NOT NULL,
             title TEXT NOT NULL,
             complete INTEGER NOT NULL DEFAULT 0 CHECK (complete IN (0, 1))
         );
         CREATE INDEX IF NOT EXISTS tasks_parent_order ON tasks(parent_id, position);
         CREATE TABLE IF NOT EXISTS planner_state (
             singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
             selected_id INTEGER,
             next_id INTEGER NOT NULL
         );",
    )?;
    Ok(db)
}

pub(crate) fn load(db: &Connection) -> rusqlite::Result<Option<LoadedState>> {
    let mut stmt = db.prepare(
        "SELECT id, parent_id, position, title, complete FROM tasks ORDER BY parent_id, position",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<i64>>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, bool>(4)?,
        ))
    })?;
    let mut pending = Vec::new();
    for row in rows {
        pending.push(row?);
    }
    if pending.is_empty() {
        return Ok(None);
    }
    let state = db
        .query_row(
            "SELECT selected_id, next_id FROM planner_state WHERE singleton = 1",
            [],
            |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?;
    let selected = state
        .as_ref()
        .and_then(|(id, _)| *id)
        .and_then(|id| u64::try_from(id).ok());
    let max_id = pending.iter().map(|row| row.0).max().unwrap_or(0).max(0) as u64;
    let next_id = state
        .and_then(|(_, id)| u64::try_from(id).ok())
        .unwrap_or(max_id + 1)
        .max(max_id + 1);
    let mut children: HashMap<i64, Vec<StoredTask>> = HashMap::new();
    let mut roots = Vec::new();
    for (id, parent, position, title, complete) in pending {
        if let Some(parent) = parent {
            children
                .entry(parent)
                .or_default()
                .push((position, id, title, complete));
        } else {
            roots.push((position, id, title, complete));
        }
    }
    roots.sort_by_key(|row| row.0);
    fn build(
        row: StoredTask,
        children: &mut HashMap<i64, Vec<StoredTask>>,
    ) -> Task {
        let (_, id, title, complete) = row;
        let mut descendants = children.remove(&id).unwrap_or_default();
        descendants.sort_by_key(|row| row.0);
        Task {
            id: id as u64,
            title,
            complete,
            children: descendants
                .into_iter()
                .map(|(_, id, title, complete)| build((0, id, title, complete), children))
                .collect(),
        }
    }
    Ok(Some((
        roots
            .into_iter()
            .map(|row| build(row, &mut children))
            .collect(),
        selected,
        next_id,
    )))
}

pub(crate) fn save(
    db: &Connection,
    roots: &[Task],
    selected: Option<u64>,
    next_id: u64,
) -> rusqlite::Result<()> {
    let tx = db.unchecked_transaction()?;
    tx.execute("DELETE FROM tasks", [])?;
    fn insert(
        tx: &rusqlite::Transaction<'_>,
        tasks: &[Task],
        parent: Option<i64>,
    ) -> rusqlite::Result<()> {
        for (position, task) in tasks.iter().enumerate() {
            let id = i64::try_from(task.id)
                .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, task.id as i64))?;
            tx.execute("INSERT INTO tasks(id, parent_id, position, title, complete) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, parent, position as i64, task.title, task.complete])?;
            insert(tx, &task.children, Some(id))?;
        }
        Ok(())
    }
    insert(&tx, roots, None)?;
    let next_id = i64::try_from(next_id)
        .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, next_id as i64))?;
    let selected = selected.map(i64::try_from).transpose().map_err(|_| {
        rusqlite::Error::IntegralValueOutOfRange(0, selected.unwrap_or_default() as i64)
    })?;
    tx.execute("INSERT INTO planner_state(singleton, selected_id, next_id) VALUES (1, ?1, ?2) ON CONFLICT(singleton) DO UPDATE SET selected_id=excluded.selected_id, next_id=excluded.next_id", params![selected, next_id])?;
    tx.commit()
}
