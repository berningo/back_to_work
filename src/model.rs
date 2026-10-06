use rusqlite::Connection;

#[derive(Clone)]
pub(crate) struct Task {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) complete: bool,
    pub(crate) children: Vec<Task>,
}

impl Task {
    fn new(id: u64, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            complete: false,
            children: Vec::new(),
        }
    }

    pub(crate) fn state(&self) -> TaskState {
        if self.children.is_empty() {
            return if self.complete {
                TaskState::Done
            } else {
                TaskState::Open
            };
        }
        let states: Vec<_> = self.children.iter().map(Task::state).collect();
        if states.iter().all(|state| *state == TaskState::Done) {
            TaskState::Done
        } else if states.iter().all(|state| *state == TaskState::Open) {
            TaskState::Open
        } else {
            TaskState::Partial
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskState {
    Open,
    Partial,
    Done,
}

#[derive(Clone)]
pub(crate) enum Action {
    AddRoot,
    AddChild(u64),
    Rename(u64),
    Delete(u64),
    Toggle(u64),
    Move(u64, u64, DropPosition),
}

#[derive(Clone, Copy)]
pub(crate) enum DropPosition {
    Before,
    Inside,
    After,
}

pub(crate) struct Dialog {
    pub(crate) action: Action,
    pub(crate) title: String,
}

pub(crate) struct Planner {
    pub(crate) roots: Vec<Task>,
    pub(crate) selected: Option<u64>,
    next_id: u64,
    pub(crate) dialog: Option<Dialog>,
    pub(crate) db: Connection,
    pub(crate) dragging: Option<u64>,
    pub(crate) search_open: bool,
    pub(crate) search_query: String,
    pub(crate) reveal_task: Option<u64>,
}

impl Planner {
    pub(crate) fn move_task(&mut self, id: u64, target: u64, position: DropPosition) {
        if id == target || Self::find(&self.roots, id)
            .is_some_and(|task| Self::find(&task.children, target).is_some()) { return; }
        let Some(task) = Self::take_task(&mut self.roots, id) else { return; };
        let moved = match position {
            DropPosition::Inside => Self::insert_child(&mut self.roots, task, target),
            DropPosition::Before => Self::insert_sibling(&mut self.roots, task, target, false),
            DropPosition::After => Self::insert_sibling(&mut self.roots, task, target, true),
        };
        if moved { self.persist(); }
    }
    fn take_task(tasks: &mut Vec<Task>, id: u64) -> Option<Task> {
        if let Some(i) = tasks.iter().position(|t| t.id == id) { return Some(tasks.remove(i)); }
        for task in tasks { if let Some(found) = Self::take_task(&mut task.children, id) { return Some(found); } }
        None
    }
    fn insert_sibling(tasks: &mut Vec<Task>, task: Task, target: u64, after: bool) -> bool {
        if let Some(i) = tasks.iter().position(|t| t.id == target) { tasks.insert(i + usize::from(after), task); return true; }
        for parent in tasks { if Self::insert_sibling(&mut parent.children, task.clone(), target, after) { return true; } }
        false
    }
    fn insert_child(tasks: &mut [Task], task: Task, target: u64) -> bool {
        for parent in tasks {
            if parent.id == target { parent.children.push(task); return true; }
            if Self::insert_child(&mut parent.children, task.clone(), target) { return true; }
        }
        false
    }
    pub(crate) fn from_database(db: Connection) -> Self {
        match crate::storage::load(&db) {
            Ok(Some((roots, selected, next_id))) => Self {
                roots,
                selected,
                next_id,
                dialog: None,
                db,
                dragging: None,
                search_open: false,
                search_query: String::new(),
                reveal_task: None,
            },
            Ok(None) => {
                let planner = Self {
                    db,
                    dragging: None,
                    search_open: false,
                    search_query: String::new(),
                    reveal_task: None,
                    ..Self::default_without_db()
                };
                planner.persist();
                planner
            }
            Err(error) => {
                eprintln!("Could not load tasks from SQLite: {error}");
                Self {
                    db,
                    dragging: None,
                    ..Self::default_without_db()
                }
            }
        }
    }

    fn default_without_db() -> Self {
        Self {
            roots: vec![Task::new(1, "Mein Projekt")],
            selected: Some(1),
            next_id: 2,
            dialog: None,
            db: Connection::open_in_memory().expect("in-memory SQLite connection"),
            dragging: None,
            search_open: false,
            search_query: String::new(),
            reveal_task: None,
        }
    }

    pub(crate) fn persist(&self) {
        if let Err(error) = crate::storage::save(&self.db, &self.roots, self.selected, self.next_id)
        {
            eprintln!("Could not save tasks to SQLite: {error}");
        }
    }
    pub(crate) fn find(tasks: &[Task], id: u64) -> Option<&Task> {
        for task in tasks {
            if task.id == id {
                return Some(task);
            }
            if let Some(found) = Self::find(&task.children, id) {
                return Some(found);
            }
        }
        None
    }

    fn find_mut(tasks: &mut [Task], id: u64) -> Option<&mut Task> {
        for task in tasks {
            if task.id == id {
                return Some(task);
            }
            if let Some(found) = Self::find_mut(&mut task.children, id) {
                return Some(found);
            }
        }
        None
    }

    fn parent_id(tasks: &[Task], id: u64) -> Option<u64> {
        for task in tasks {
            if task.children.iter().any(|child| child.id == id) {
                return Some(task.id);
            }
            if let Some(found) = Self::parent_id(&task.children, id) {
                return Some(found);
            }
        }
        None
    }

    pub(crate) fn apply(&mut self, action: Action, title: String) {
        match action {
            Action::AddRoot => {
                let id = self.next_id;
                self.next_id += 1;
                self.roots.push(Task::new(id, title));
                self.selected = Some(id);
            }
            Action::AddChild(parent) => {
                let id = self.next_id;
                self.next_id += 1;
                if let Some(task) = Self::find_mut(&mut self.roots, parent) {
                    task.children.push(Task::new(id, title));
                    self.selected = Some(id);
                }
            }
            Action::Rename(id) => {
                if let Some(task) = Self::find_mut(&mut self.roots, id) {
                    task.title = title;
                }
            }
            Action::Delete(id) => {
                if let Some(parent) = Self::parent_id(&self.roots, id) {
                    if let Some(task) = Self::find_mut(&mut self.roots, parent) {
                        task.children.retain(|child| child.id != id);
                    }
                    self.selected = Some(parent);
                } else {
                    self.roots.retain(|task| task.id != id);
                    self.selected = self.roots.first().map(|task| task.id);
                }
            }
            Action::Toggle(id) => {
                if let Some(task) = Self::find_mut(&mut self.roots, id)
                    && task.children.is_empty()
                {
                    task.complete = !task.complete;
                }
            }
            Action::Move(_, _, _) => {}
        }
        self.persist();
    }

    pub(crate) fn dialog(&mut self, action: Action) {
        let title = if let Action::Rename(id) = &action {
            Self::find(&self.roots, *id)
                .map(|task| task.title.clone())
                .unwrap_or_default()
        } else {
            String::new()
        };
        self.dialog = Some(Dialog { action, title });
    }
}

pub(crate) fn count_tasks(tasks: &[Task]) -> (usize, usize) {
    tasks.iter().fold((0, 0), |(total, done), task| {
        let (child_total, child_done) = count_tasks(&task.children);
        if task.children.is_empty() {
            (total + 1, done + usize::from(task.complete))
        } else {
            (total + child_total, done + child_done)
        }
    })
}
