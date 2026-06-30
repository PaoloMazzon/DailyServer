use std::{sync::{LazyLock, atomic::{AtomicBool, Ordering::Relaxed}}, time::Duration};
use tokio::task::spawn;
use spdlog::prelude::*;

static GLOBAL_KILL_SIGNAL: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));
static KILL_SIGNAL_TIMEOUT: Duration = Duration::from_secs(5);

/// Kills the program "safely" by setting the global "kill" signal to true
/// (it's up to each thread to watch out for the kill signal and handle it
/// accordingly). This function also spawns a "kill thread" that will nuke
/// the entire program in 5 seconds.
pub fn kill_program() {
    GLOBAL_KILL_SIGNAL.store(true, Relaxed);
    spawn(async {
        info!("Received kill signal, spawned thread to kill process in {} seconds.", KILL_SIGNAL_TIMEOUT.as_secs());
        tokio::time::sleep(KILL_SIGNAL_TIMEOUT).await;
        panic!("Kill timeout exceeded. Killing program.");
    });
}

/// Program cannot safely continue so it must explode, also logs
#[allow(dead_code)]
pub fn instant_kill_program() -> ! {
    panic!("[FATAL] Instantly killing program");
}

/// Returns true if any thread anywhere has requested the program be terminated.
/// The program will be forcibly aborted within 5 seconds of the kill signal
/// being received so any cleanup should be prioritized after this is true.
pub fn kill_signal_received() -> bool {
    GLOBAL_KILL_SIGNAL.load(Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kill_signal() {
        assert_eq!(kill_signal_received(), false);
        kill_program();
        assert_eq!(kill_signal_received(), true);
    }

    #[test]
    #[should_panic]
    fn instant_kill_instant_kills() {
        instant_kill_program();
    }
}