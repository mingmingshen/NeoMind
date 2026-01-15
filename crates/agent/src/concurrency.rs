//! Per-Session Concurrency Limiting
//!
//! This module provides session-aware concurrency limiting for LLM requests.
//! It prevents a single session from monopolizing all available permits.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use edge_ai_core::SessionId;

/// Default per-session concurrent request limit.
pub const DEFAULT_PER_SESSION_LIMIT: usize = 2;

/// Global concurrent request limit (shared across all sessions).
pub const DEFAULT_GLOBAL_LIMIT: usize = 3;

/// Session-aware concurrency limiter.
///
/// This limiter ensures:
/// 1. No single session exceeds its per-session limit
/// 2. Total concurrent requests across all sessions don't exceed global limit
#[derive(Clone)]
pub struct SessionConcurrencyLimiter {
    inner: Arc<SessionConcurrencyLimiterInner>,
}

struct SessionConcurrencyLimiterInner {
    /// Global limit across all sessions
    global_limit: usize,
    /// Per-session limit
    per_session_limit: usize,
    /// Current global usage
    global_current: AtomicUsize,
    /// Per-session current usage
    session_usage: tokio::sync::RwLock<HashMap<SessionId, SessionUsage>>,
}

/// Per-session usage tracking.
struct SessionUsage {
    /// Current number of active requests
    current: AtomicUsize,
    /// Maximum allowed for this session
    max: usize,
}

impl SessionUsage {
    fn new(max: usize) -> Self {
        Self {
            current: AtomicUsize::new(0),
            max,
        }
    }
}

/// A permit that releases when dropped.
pub struct SessionPermit {
    session_id: SessionId,
    limiter: Arc<SessionConcurrencyLimiterInner>,
}

impl Drop for SessionPermit {
    fn drop(&mut self) {
        // Release from session tracking
        if let Ok(mut sessions) = self.limiter.session_usage.try_write() {
            if let Some(usage) = sessions.get(&self.session_id) {
                usage.current.fetch_sub(1, Ordering::Relaxed);
            }
        }

        // Release from global tracking
        self.limiter.global_current.fetch_sub(1, Ordering::Relaxed);
    }
}

impl SessionConcurrencyLimiter {
    /// Create a new session-aware concurrency limiter.
    pub fn new(global_limit: usize, per_session_limit: usize) -> Self {
        Self {
            inner: Arc::new(SessionConcurrencyLimiterInner {
                global_limit,
                per_session_limit,
                global_current: AtomicUsize::new(0),
                session_usage: tokio::sync::RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Create with default limits.
    pub fn default_limits() -> Self {
        Self::new(DEFAULT_GLOBAL_LIMIT, DEFAULT_PER_SESSION_LIMIT)
    }

    /// Try to acquire a permit for a session.
    ///
    /// Returns Some(permit) if successful, None if at limit.
    pub fn try_acquire(&self, session_id: &SessionId) -> Option<SessionPermit> {
        // Check session limit first
        {
            let sessions = self.inner.session_usage.try_read().ok()?;
            let session_current = sessions
                .get(session_id)
                .map(|u| u.current.load(Ordering::Relaxed))
                .unwrap_or(0);

            if session_current >= self.inner.per_session_limit {
                return None; // Session at limit
            }
        }

        // Check global limit
        let global_current = self.inner.global_current.load(Ordering::Relaxed);
        if global_current >= self.inner.global_limit {
            return None; // Global at limit
        }

        // Try to increment global counter
        let mut current = global_current;
        loop {
            if current >= self.inner.global_limit {
                return None;
            }
            match self.inner.global_current.compare_exchange_weak(
                current,
                current + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_current) => current = new_current,
            }
        }

        // Increment session counter
        {
            let mut sessions = self.inner.session_usage.try_write().ok()?;
            let usage = sessions
                .entry(session_id.clone())
                .or_insert_with(|| SessionUsage::new(self.inner.per_session_limit));

            // Double-check session limit (could have changed)
            if usage.current.load(Ordering::Relaxed) >= self.inner.per_session_limit {
                // Rollback global increment
                self.inner.global_current.fetch_sub(1, Ordering::Relaxed);
                return None;
            }

            usage.current.fetch_add(1, Ordering::Relaxed);
        }

        Some(SessionPermit {
            session_id: session_id.clone(),
            limiter: self.inner.clone(),
        })
    }

    /// Acquire a permit, waiting until one is available.
    pub async fn acquire(&self, session_id: &SessionId) -> SessionPermit {
        loop {
            if let Some(permit) = self.try_acquire(session_id) {
                return permit;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Get current statistics.
    pub async fn stats(&self) -> ConcurrencyStats {
        let global_current = self.inner.global_current.load(Ordering::Relaxed);
        let sessions = self.inner.session_usage.read().await;

        let per_session_stats: HashMap<SessionId, usize> = sessions
            .iter()
            .map(|(id, usage)| (id.clone(), usage.current.load(Ordering::Relaxed)))
            .collect();

        ConcurrencyStats {
            global_limit: self.inner.global_limit,
            global_current,
            global_available: self.inner.global_limit.saturating_sub(global_current),
            per_session_limit: self.inner.per_session_limit,
            per_session_usage: per_session_stats,
        }
    }

    /// Remove session tracking (call when session ends).
    pub async fn remove_session(&self, session_id: &SessionId) {
        let mut sessions = self.inner.session_usage.write().await;
        sessions.remove(session_id);
    }
}

/// Concurrency statistics.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConcurrencyStats {
    /// Global limit
    pub global_limit: usize,
    /// Current global usage
    pub global_current: usize,
    /// Available global slots
    pub global_available: usize,
    /// Per-session limit
    pub per_session_limit: usize,
    /// Per-session current usage
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub per_session_usage: HashMap<SessionId, usize>,
}

/// Fallback to simple global limiter (for backward compatibility).
///
/// This is used when session_id is not available.
#[derive(Clone)]
pub struct GlobalConcurrencyLimiter {
    current: Arc<AtomicUsize>,
    max: usize,
}

impl GlobalConcurrencyLimiter {
    pub fn new(max: usize) -> Self {
        Self {
            current: Arc::new(AtomicUsize::new(0)),
            max,
        }
    }

    pub fn try_acquire(&self) -> Option<GlobalPermit> {
        let mut current = self.current.load(Ordering::Relaxed);
        loop {
            if current >= self.max {
                return None;
            }
            match self.current.compare_exchange_weak(
                current,
                current + 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Some(GlobalPermit {
                        limiter: self.current.clone(),
                    });
                }
                Err(new_current) => current = new_current,
            }
        }
    }

    pub async fn acquire(&self) -> GlobalPermit {
        loop {
            if let Some(permit) = self.try_acquire() {
                return permit;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    pub fn available(&self) -> usize {
        self.max
            .saturating_sub(self.current.load(Ordering::Relaxed))
    }
}

/// A global permit that releases when dropped.
pub struct GlobalPermit {
    limiter: Arc<AtomicUsize>,
}

impl Drop for GlobalPermit {
    fn drop(&mut self) {
        self.limiter.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_concurrency_basic() {
        let limiter = SessionConcurrencyLimiter::new(3, 2);
        let session_id = edge_ai_core::SessionId::from("session_1");

        // First acquire should succeed
        let permit1 = limiter.try_acquire(&session_id);
        assert!(permit1.is_some());

        // Second acquire should succeed
        let permit2 = limiter.try_acquire(&session_id);
        assert!(permit2.is_some());

        // Third acquire should fail (per-session limit of 2)
        let permit3 = limiter.try_acquire(&session_id);
        assert!(permit3.is_none());

        // Drop first permit
        drop(permit1);

        // Now third acquire should succeed
        let permit3 = limiter.try_acquire(&session_id);
        assert!(permit3.is_some());
    }

    #[tokio::test]
    async fn test_global_concurrency_limit() {
        let limiter = SessionConcurrencyLimiter::new(2, 3); // Global limit lower than per-session
        let session1 = edge_ai_core::SessionId::from("session_1");
        let session2 = edge_ai_core::SessionId::from("session_2");

        // Session 1 can get 2 permits
        let p1a = limiter.try_acquire(&session1);
        let p1b = limiter.try_acquire(&session1);
        assert!(p1a.is_some());
        assert!(p1b.is_some());

        // Session 2 should be blocked by global limit
        let p2 = limiter.try_acquire(&session2);
        assert!(p2.is_none());

        // Drop one permit from session 1
        drop(p1a);

        // Now session 2 can get a permit
        let p2 = limiter.try_acquire(&session2);
        assert!(p2.is_some());
    }

    #[tokio::test]
    async fn test_stats() {
        let limiter = SessionConcurrencyLimiter::new(5, 2);
        let session1 = edge_ai_core::SessionId::from("session_1");
        let session2 = edge_ai_core::SessionId::from("session_2");

        let _p1a = limiter.try_acquire(&session1);
        let _p1b = limiter.try_acquire(&session1);
        let _p2a = limiter.try_acquire(&session2);

        let stats = limiter.stats().await;
        assert_eq!(stats.global_current, 3);
        assert_eq!(stats.global_available, 2);
        // Note: per_session_usage uses SessionId keys
        assert!(stats.per_session_usage.len() == 2);
    }

    #[tokio::test]
    async fn test_remove_session() {
        let limiter = SessionConcurrencyLimiter::new(5, 2);
        let session_id = edge_ai_core::SessionId::from("session_1");

        let _p1 = limiter.try_acquire(&session_id);
        limiter.remove_session(&session_id).await;

        let stats = limiter.stats().await;
        assert!(stats.per_session_usage.is_empty());
    }

    #[tokio::test]
    async fn test_global_limiter() {
        let limiter = GlobalConcurrencyLimiter::new(2);

        let p1 = limiter.try_acquire();
        assert!(p1.is_some());

        let p2 = limiter.try_acquire();
        assert!(p2.is_some());

        let p3 = limiter.try_acquire();
        assert!(p3.is_none());

        assert_eq!(limiter.available(), 0);

        drop(p1);
        assert_eq!(limiter.available(), 1);
    }
}
