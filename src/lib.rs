use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

struct ThreadIdPool {
    used_ids: AtomicU8,
}

impl ThreadIdPool {
    fn global() -> &'static ThreadIdPool {
        static POOL: OnceLock<ThreadIdPool> = OnceLock::new();
        POOL.get_or_init(|| ThreadIdPool {
            used_ids: AtomicU8::new(0),
        })
    }

    fn acquire(&self) -> Result<u8, &'static str> {
        loop {
            let current = self.used_ids.load(Ordering::SeqCst);
            if !current == 0 {
                return Err("No available thread IDs (max 8)");
            }
            for i in 0..8 {
                let mask = 1 << i;
                if current & mask == 0 {
                    let new_value = current | mask;
                    if self
                        .used_ids
                        .compare_exchange(current, new_value, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        return Ok(i);
                    }
                }
            }
        }
    }

    fn release(&self, id: u8) {
        let mask = !(1 << id);
        self.used_ids.fetch_and(mask, Ordering::SeqCst);
    }

    fn is_full(&self) -> bool {
        !self.used_ids.load(Ordering::SeqCst) == 0
    }
}

pub struct Switflake {
    node_id: u64,
    thread_id: u8,
    local_counter: u8,
}

impl Switflake {
    pub fn new(node_id: u64) -> Result<Self, &'static str> {
        let pool = ThreadIdPool::global();
        if pool.is_full() {
            return Err("Thread pool full (max 8 simultaneous threads)");
        }
        let thread_id = pool.acquire()?;
        Ok(Switflake {
            node_id: node_id & 0xFFF,
            thread_id,
            local_counter: 0,
        })
    }

    #[inline]
    pub fn generate_id(&mut self) -> Result<u64, &'static str> {
        if self.local_counter == 0xFF {
            return Err("Sequence limit reached for this millisecond");
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Time went backwards")?
            .as_millis() as u64
            & 0x1FFFFFFFFFF;
        let sequence = (self.thread_id as u64) << 8 | (self.local_counter as u64);
        let id = (timestamp << 23) | (self.node_id << 11) | (sequence & 0x7FF);
        self.local_counter += 1;
        Ok(id)
    }
}

impl Drop for Switflake {
    fn drop(&mut self) {
        ThreadIdPool::global().release(self.thread_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::thread;

    #[test]
    fn test_unique_ids_single_thread() {
        let mut swit = Switflake::new(1).expect("Failed to create Switflake");
        let mut ids = HashSet::new();
        for _ in 0..100 {
            let id = swit.generate_id().expect("Failed to generate ID");
            assert!(ids.insert(id), "Duplicate ID found: {}", id);
        }
        assert_eq!(ids.len(), 100, "Not all IDs were unique");
    }

    #[test]
    fn test_pool_full_and_reuse() {
        let mut handles = Vec::new();
        // 스레드 8개
        for _ in 0..8 {
            let mut swit = match Switflake::new(1) {
                Ok(swit) => swit,
                Err(e) => panic!("Unexpected error during setup: {}", e),
            };
            handles.push(thread::spawn(move || {
                let _ = swit.generate_id().expect("Failed to generate ID");
            }));
        }

        // 스레드 꽉차면
        for handle in handles {
            handle.join().expect("Thread join failed");
        }

        // 생서앟면 에러
        assert!(Switflake::new(1).is_err(), "Should fail when pool is full");

        // 근데 종료하면 생성이 가능
        let mut swit = Switflake::new(1).expect("Failed to create Switflake after reuse");
        assert!(swit.generate_id().is_ok());
    }

    #[test]
    fn test_sequence_limit() {
        let mut swit = Switflake::new(1).expect("Failed to create Switflake");
        for _ in 0..255 {
            let _ = swit.generate_id().expect("Failed to generate ID");
        }
        assert!(
            swit.generate_id().is_err(),
            "Should error at sequence limit"
        );
    }

    #[test]
    fn test_multi_thread_unique_ids() {
        let mut handles = Vec::new();
        let mut all_ids = HashSet::new();
        for _ in 0..8 {
            let mut swit = Switflake::new(1).expect("Failed to create Switflake");
            handles.push(thread::spawn(move || {
                let mut ids = Vec::new();
                for _ in 0..64 {
                    if let Ok(id) = swit.generate_id() {
                        ids.push(id);
                    }
                }
                ids
            }));
        }
        for handle in handles {
            let ids = handle.join().expect("Thread join failed");
            for id in ids {
                assert!(all_ids.insert(id), "Duplicate ID found: {}", id);
            }
        }
        assert_eq!(
            all_ids.len(),
            8 * 64,
            "Not all IDs were unique across threads"
        );
    }
}
