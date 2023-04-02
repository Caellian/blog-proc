use std::sync::Arc;

use async_lock::RwLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Progress {
    Tasks {
        done: usize,
        total: usize,
    },
    Floating {
        /// Progress value in range [0.0, 1.0]
        value: f32,
    },
    Undetermined,
}

#[derive(Debug, Clone)]
pub struct Status {
    pub progress: Arc<RwLock<Progress>>,
}
unsafe impl Send for Status {}
unsafe impl Sync for Status {}

impl Status {
    #[inline]
    pub async fn update_progress(&self, update: Progress) {
        if *(self.progress.read().await) != update {
            let mut w = self.progress.write().await;
            *w = update;
        }
    }
}
