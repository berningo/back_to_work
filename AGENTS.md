# Module guide for agents

- Language: Rust 1.99.0 stable (edition 2024).
- Framework: eframe/egui with the wgpu renderer; persistent data is stored in SQLite via rusqlite.

- `src/main.rs`: Starts the native eframe application, creates the data directory, opens the SQLite database, and initializes the planner.
- `src/model.rs`: Defines tasks, planner state, task actions, derived completion status, and the operations that update the task tree.
- `src/storage.rs`: Chooses the database path and creates, loads, and transactionally saves the task tree and planner state in SQLite.
- `src/ui.rs`: Implements the egui interface, including task rows, selection, keyboard and context-menu actions, dialogs, and drag-and-drop reordering.
