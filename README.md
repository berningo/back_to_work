# Aufgabenplaner

Native German-language hierarchical task planner rewritten in Rust. The UI is drawn by egui using its wgpu renderer.

## Build and run

Install Rust with rustup, then run:

```powershell
cargo run --release
```

## Controls

- Select a task and use **＋ Unteraufgabe** to add a child task.
- **＋ Aufgabe** or `Ctrl+N` adds a top-level task.
- `Ctrl+Shift+N` adds a child to the selected task.
- Click a leaf task's status icon or press `Space` to toggle completion. Parent status is derived from its children.
- Double-click a task or press `F2` to rename it.
- Press `Delete` to remove the selected task and its descendants.
- Right-click a task for the same actions.

Tasks are stored in SQLite and loaded when the application starts. The database is
`%APPDATA%\back_to_work\tasks.sqlite3` on Windows (or the platform data directory on
other systems). The `tasks` table stores each task with a parent ID and sibling order;
`planner_state` stores the selected task and next ID. Each task change is committed
in one transaction.
