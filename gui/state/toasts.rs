use std::collections::VecDeque;
use std::time::{Duration, Instant};

const TOAST_TTL: Duration = Duration::from_secs(6);

pub struct ErrorToast {
    pub message: String,
    pub shown_since: Instant,
}

#[derive(Default)]
pub struct ToastQueue {
    pub items: VecDeque<ErrorToast>,
}

impl ToastQueue {
    pub fn push(&mut self, msg: impl Into<String>) {
        // Evict expired entries while adding, to bound queue size.
        self.gc();
        self.items.push_back(ErrorToast {
            message: msg.into(),
            shown_since: Instant::now(),
        });
    }

    /// Remove expired toasts. Returns `true` if any live toasts remain.
    pub fn gc(&mut self) -> bool {
        self.items.retain(|t| t.shown_since.elapsed() < TOAST_TTL);
        !self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_adds_toast() {
        let mut q = ToastQueue::default();
        q.push("test error");
        assert_eq!(q.items.len(), 1);
        assert_eq!(q.items[0].message, "test error");
    }

    #[test]
    fn gc_removes_only_expired() {
        let mut q = ToastQueue::default();
        q.items.push_back(ErrorToast {
            message: "old".into(),
            shown_since: Instant::now() - Duration::from_secs(7),
        });
        q.items.push_back(ErrorToast {
            message: "fresh".into(),
            shown_since: Instant::now(),
        });
        let has_live = q.gc();
        assert!(has_live);
        assert_eq!(q.items.len(), 1);
        assert_eq!(q.items[0].message, "fresh");
    }
}
