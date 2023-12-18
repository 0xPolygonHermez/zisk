use std::sync::{Mutex, Arc};

#[derive(Debug, PartialEq)]
pub enum TaskStatus {
    Pending,
    Finished,
}

pub trait TaskPayload: Send + Sync {
}

pub struct Task {
    pub src: String,
    pub dst: String,
    pub payload: Box<dyn TaskPayload>,
    pub status: TaskStatus,
}

impl Task {
    pub fn new(src: String, dst: String, payload: Box<dyn TaskPayload>) -> Self {
        Task {
            src,
            dst,
            payload,
            status: TaskStatus::Pending,
        }
    }
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task")
            .field("src", &self.src)
            .field("dst", &self.dst)
            .field("payload", &"TaskPayload")
            .field("status", &self.status)
            .finish()
    }
}

#[derive(Debug)]
pub struct TasksTable {
    tasks: Arc<Mutex<Vec<Task>>>,
}

#[derive(Debug, PartialEq)]
pub enum TasksTableError {
    NotFound,
}

impl TasksTable {
    pub fn new() -> Self {
        TasksTable { tasks: Arc::new(Mutex::new(Vec::new())) }
    }

    pub fn add_task(&self, src: String, dst: String, payload: Box<dyn TaskPayload>) {
        self.tasks.lock().unwrap().push(Task::new(src, dst, payload));
    }

    pub fn resolve_task(&self, task_id: usize) -> Result<(), TasksTableError> {
        let mut tasks = self.tasks.lock().unwrap();
        if task_id < tasks.len() {
            tasks[task_id].status = TaskStatus::Finished;
            Ok(())
        } else {
            Err(TasksTableError::NotFound)
        }
    }

    pub fn get_tasks_id_by_src(&self, src: &str) -> Vec<usize> {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter().enumerate().filter(|(_, task)| task.src == src.to_string()).map(|(id, _)| id).collect()
    }

    pub fn get_tasks_id_by_dst(&self, dst: &str) -> Vec<usize> {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter().enumerate().filter(|(_, task)| task.dst == dst.to_string()).map(|(id, _)| id).collect()
    }

    pub fn get_num_tasks(&self) -> usize {
        let tasks = self.tasks.lock().unwrap();
        tasks.len()
    }

    pub fn get_num_pending_tasks(&self) -> usize {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter().filter(|&task| task.status == TaskStatus::Pending).count()
    }

    pub fn get_num_finished_tasks(&self) -> usize {
        let tasks = self.tasks.lock().unwrap();
        tasks.iter().filter(|&task| task.status == TaskStatus::Finished).count()
    }
}

impl Clone for TasksTable {
    fn clone(&self) -> Self {
        TasksTable {
            tasks: Arc::clone(&self.tasks),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyPayload;

    impl TaskPayload for DummyPayload {}

    #[test]
    fn test_tasks_table_creation() {
        let tasks_table = TasksTable::new();
        assert_eq!(tasks_table.get_num_tasks(), 0);
        assert_eq!(tasks_table.get_num_pending_tasks(), 0);
        assert_eq!(tasks_table.get_num_finished_tasks(), 0);
    }

    #[test]
    fn test_tasks_table_add_task() {
        let tasks_table = TasksTable::new();
        tasks_table.add_task("src".to_string(), "dst".to_string(), Box::new(DummyPayload));
        assert_eq!(tasks_table.get_num_tasks(), 1);
        assert_eq!(tasks_table.get_num_pending_tasks(), 1);
        assert_eq!(tasks_table.get_num_finished_tasks(), 0);
    }

    #[test]
    fn test_tasks_table_resolve_task() {
        let tasks_table = TasksTable::new();
        let result = tasks_table.resolve_task(0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), TasksTableError::NotFound);
        tasks_table.add_task("src".to_string(), "dst".to_string(), Box::new(DummyPayload));
        assert_eq!(tasks_table.get_num_pending_tasks(), 1);
        assert_eq!(tasks_table.get_num_finished_tasks(), 0);
        tasks_table.resolve_task(0).unwrap();
        assert_eq!(tasks_table.get_num_pending_tasks(), 0);
        assert_eq!(tasks_table.get_num_finished_tasks(), 1);
    }

    #[test]
    fn test_tasks_table_clone() {
        let tasks_table1 = TasksTable::new();
        tasks_table1.add_task("src".to_string(), "dst".to_string(), Box::new(DummyPayload));
        let tasks_table2 = tasks_table1.clone();

        assert_eq!(tasks_table2.get_num_tasks(), 1);
        assert_eq!(tasks_table2.get_num_pending_tasks(), 1);
        assert_eq!(tasks_table2.get_num_finished_tasks(), 0);
    }
}

#[cfg(test)]
mod thread_tests {
    use super::*;
    use std::thread;

    struct DummyPayload;

    impl TaskPayload for DummyPayload {}

    #[test]
    fn test_shared_tasks_table_between_threads() {
        let shared_tasks_table = Arc::new(TasksTable::new());

        // Clone a reference to the shared_tasks_table for each thread
        let thread1_tasks_table = Arc::clone(&shared_tasks_table);
        let thread2_tasks_table = Arc::clone(&shared_tasks_table);

        // Spawn two threads that manipulate the shared tasks table
        let handle1 = thread::spawn(move || {
            thread1_tasks_table.add_task("src1".to_string(), "dst1".to_string(), Box::new(DummyPayload));
        });

        let handle2 = thread::spawn(move || {
            thread2_tasks_table.add_task("src2".to_string(), "dst2".to_string(), Box::new(DummyPayload));
        });

        // Wait for both threads to finish
        handle1.join().unwrap();
        handle2.join().unwrap();

        // Check the final state of the tasks table
        assert_eq!(shared_tasks_table.get_num_tasks(), 2);
        assert_eq!(shared_tasks_table.get_num_pending_tasks(), 2);
        assert_eq!(shared_tasks_table.get_num_finished_tasks(), 0);
    }

    #[test]
    fn test_shared_tasks_table_with_resolving_tasks() {
        let shared_tasks_table = Arc::new(TasksTable::new());

        // Clone a reference to the shared_tasks_table for each thread
        let thread1_tasks_table = Arc::clone(&shared_tasks_table);
        let thread2_tasks_table = Arc::clone(&shared_tasks_table);

        // Spawn two threads that manipulate the shared tasks table
        let handle1 = thread::spawn(move || {
            thread1_tasks_table.add_task("src1".to_string(), "dst1".to_string(), Box::new(DummyPayload));
            let get_tasks_id_by_src = thread1_tasks_table.get_tasks_id_by_src("src1");
            assert_eq!(get_tasks_id_by_src.len(), 1);
            for task_id in get_tasks_id_by_src {
                thread1_tasks_table.resolve_task(task_id).unwrap();
            }
        });

        let handle2 = thread::spawn(move || {
            thread2_tasks_table.add_task("src2".to_string(), "dst2".to_string(), Box::new(DummyPayload));
            let get_tasks_id_by_src = thread2_tasks_table.get_tasks_id_by_src("src2");
            assert_eq!(get_tasks_id_by_src.len(), 1);
            for task_id in get_tasks_id_by_src {
                thread2_tasks_table.resolve_task(task_id).unwrap();
            }
        });

        // Wait for both threads to finish
        handle1.join().unwrap();
        handle2.join().unwrap();

        // Check the final state of the tasks table
        assert_eq!(shared_tasks_table.get_num_tasks(), 2);
        println!("tasks table {:?}", shared_tasks_table);
        assert_eq!(shared_tasks_table.get_num_pending_tasks(), 0);
        assert_eq!(shared_tasks_table.get_num_finished_tasks(), 2);
    }
}
