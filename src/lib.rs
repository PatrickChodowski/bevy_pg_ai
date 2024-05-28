mod pg_jobs;
mod pg_tasks;

mod utils;

pub mod prelude {
    pub use crate::pg_tasks::{JobTasks, Task, TaskStatus, TaskData, TasksPlugin, SPAWN_TASK_ID, DESPAWN_TASK_ID};
    pub use crate::pg_jobs::{PGJobsPlugin, JobCatalog, Jobs, JobSchedule, Job};  
}

pub mod tasks {
    pub use crate::pg_tasks::{Task, SpawnTask, DespawnTask, 
                              RotateTask, MoveTask, WaitTask,
                              TeleportTask, HideTask, ShowTask, 
                              DecisionTask, LoopTask};
}