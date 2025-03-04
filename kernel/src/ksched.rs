use alloc::{vec, vec::Vec};
use pasts::{Executor, Loop, prelude::*};

use crate::println;

pub type TaskNotify<'a> = BoxNotify<'a, TaskResult>;

pub trait TaskFn {
    async fn run(self) -> TaskResult;
}

#[derive(Debug)]
pub enum TaskResult {
    Exit,
}

pub struct Task {
    pub pid: usize,
    pub notify: TaskNotify<'static>,
}

impl Task {
    pub fn new(func: impl Future<Output = TaskResult> + Send + 'static, pid: usize) -> Self {
        Self {
            notify: Box::pin(func.fuse()),
            pid,
        }
    }

    pub fn run(self) -> TaskNotify<'static> {
        self.notify
    }
}

impl Notify for Task {
    type Event = TaskResult;

    fn poll_next(self: Pin<&mut Self>, t: &mut core::task::Context<'_>) -> Poll<Self::Event> {
        self.get_mut().notify.as_mut().poll_next(t)
    }
}

pub struct KernelScheduler {
    tasks: Vec<Task>,
}

impl KernelScheduler {
    fn completed(&mut self, (id, val): (usize, TaskResult)) -> Poll<bool> {
        println!("Received message from completed task: {val:?}");

        self.tasks.swap_remove(id);

        if self.tasks.is_empty() {
            Ready(true)
        } else {
            Pending
        }
    }

    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    pub fn with_task(mut self, task: Task) -> Self {
        self.tasks.push(task);
        self
    }

    pub async fn run(&mut self) {
        let l = Loop::new(self);

        l.on(|s| &mut s.tasks[..], Self::completed).await;
    }

    pub fn block_on_run(mut self) {
        Executor::default().block_on(async move {
            self.run().await;
        });
    }
}
