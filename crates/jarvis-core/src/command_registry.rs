use parking_lot::RwLockReadGuard;

use crate::{JCommandsList, COMMANDS_LIST};

/// Load (atomically replace) the entire command registry.
///
/// This is the ONLY supported write operation — the whole `Vec<JCommandsList>`
/// is replaced under an exclusive write lock, preventing partial mutations.
pub fn load(commands: Vec<JCommandsList>) {
    let count = commands.len();
    *COMMANDS_LIST.write() = commands;
    log::info!("CommandRegistry: loaded {} pack(s)", count);
}

/// Acquire a shared read guard over the registry.
///
/// Multiple readers can hold guards simultaneously.
/// The guard is released when it goes out of scope.
pub fn read() -> RwLockReadGuard<'static, Vec<JCommandsList>> {
    COMMANDS_LIST.read()
}

/// Return a cloned snapshot of the entire registry.
///
/// Use when you need an owned `Vec` (e.g., to pass to a background thread).
pub fn get_snapshot() -> Vec<JCommandsList> {
    COMMANDS_LIST.read().to_vec()
}

/// Returns `true` if at least one command pack is loaded.
pub fn is_loaded() -> bool {
    !COMMANDS_LIST.read().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::JCommandsList;
    use once_cell::sync::Lazy;
    use parking_lot::Mutex;

    // Serialise tests that touch the process-global COMMANDS_LIST.
    static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn make_pack(id: &str) -> JCommandsList {
        JCommandsList {
            path: std::path::PathBuf::from(id),
            commands: vec![],
        }
    }

    /// Reset registry to empty between tests.
    fn teardown() {
        *crate::COMMANDS_LIST.write() = vec![];
    }

    #[test]
    fn test_load_stores_commands() {
        let _g = TEST_LOCK.lock();
        load(vec![make_pack("test_pack_a")]);
        assert!(is_loaded());
        teardown();
    }

    #[test]
    fn test_load_is_atomic_replacement() {
        let _g = TEST_LOCK.lock();
        load(vec![make_pack("old_pack")]);
        load(vec![make_pack("new_pack_1"), make_pack("new_pack_2")]);
        let snapshot = get_snapshot();
        assert_eq!(snapshot.len(), 2);
        teardown();
    }

    #[test]
    fn test_get_snapshot_returns_all() {
        let _g = TEST_LOCK.lock();
        load(vec![make_pack("snap_a"), make_pack("snap_b")]);
        let snapshot = get_snapshot();
        assert_eq!(snapshot.len(), 2);
        teardown();
    }

    #[test]
    fn test_read_guard_gives_access() {
        let _g = TEST_LOCK.lock();
        load(vec![make_pack("guard_pack")]);
        let guard = read();
        assert!(!guard.is_empty());
        drop(guard);
        teardown();
    }

    #[test]
    fn test_is_loaded_true_after_load() {
        let _g = TEST_LOCK.lock();
        load(vec![make_pack("loaded_pack")]);
        assert!(is_loaded());
        teardown();
    }

    #[test]
    fn test_is_loaded_false_after_teardown() {
        let _g = TEST_LOCK.lock();
        load(vec![make_pack("some_pack")]);
        teardown();
        assert!(!is_loaded());
    }
}
