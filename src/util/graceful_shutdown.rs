use std::{process::abort, sync::{LazyLock, atomic::{AtomicBool, Ordering::Relaxed}}, time::Duration};
use tokio::task::spawn;
use spdlog::prelude::*;

static GLOBAL_KILL_SIGNAL: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));
static KILL_SIGNAL_TIMEOUT: Duration = Duration::from_secs(5);

pub fn kill_program() {
    GLOBAL_KILL_SIGNAL.store(true, Relaxed);
    spawn(async {
        info!("Received kill signal, spawned thread to kill process in {} seconds.", KILL_SIGNAL_TIMEOUT.as_secs());
        tokio::time::sleep(KILL_SIGNAL_TIMEOUT).await;
        error!("Kill timeout exceeded. Killing program.");
        abort();
    });
}

pub fn kill_signal_received() -> bool {
    GLOBAL_KILL_SIGNAL.load(Relaxed)
}

#[cfg(test)]
mod tests {
    use crate::util::graceful_shutdown::{kill_program, kill_signal_received};

    #[tokio::test]
    async fn test_kill_signal() {
        assert_eq!(kill_signal_received(), false);
        kill_program();
        assert_eq!(kill_signal_received(), true);
    }
}