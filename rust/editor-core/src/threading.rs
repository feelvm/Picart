//! UI / Render / IO / AI queues. Realtime render path has priority;
//! decode/encode/export/AI never run on UI or render threads.

use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueKind { Render, Io, Ai }

pub struct EngineQueues {
    pub render_tx: Sender<Job>,
    pub io_tx: Sender<Job>,
    pub ai_tx: Sender<Job>,
    pub render_rx: Receiver<Job>,
    pub io_rx: Receiver<Job>,
    pub ai_rx: Receiver<Job>,
}

pub struct Job {
    pub kind: QueueKind,
    pub label: String,
    pub task: Box<dyn FnOnce() + Send + 'static>,
}

impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job").field("kind", &self.kind).field("label", &self.label).finish()
    }
}

impl EngineQueues {
    pub fn new() -> Self {
        let (render_tx, render_rx) = channel();
        let (io_tx, io_rx) = channel();
        let (ai_tx, ai_rx) = channel();
        Self { render_tx, io_tx, ai_tx, render_rx, io_rx, ai_rx }
    }
    pub fn post<F: FnOnce() + Send + 'static>(&self, kind: QueueKind, label: impl Into<String>, f: F) {
        let job = Job { kind, label: label.into(), task: Box::new(f) };
        let _ = match kind {
            QueueKind::Render => self.render_tx.send(job),
            QueueKind::Io => self.io_tx.send(job),
            QueueKind::Ai => self.ai_tx.send(job),
        };
    }
}

impl Default for EngineQueues {
    fn default() -> Self { Self::new() }
}
