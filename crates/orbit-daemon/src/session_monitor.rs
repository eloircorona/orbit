use orbit_core::session::Session;
use std::time::Duration;
use tokio::{sync::broadcast, time::sleep};
use tracing::debug;

pub async fn run_cleanup_loop(
    interval: Duration,
    mut shutdown: broadcast::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = sleep(interval) => {
                let cleaned = clean_dead_sessions();
                if cleaned > 0 {
                    debug!("session monitor: cleaned {cleaned} dead session(s)");
                }
            }
            _ = shutdown.recv() => break,
        }
    }
}

fn clean_dead_sessions() -> usize {
    let sessions = Session::load_all();
    let mut count = 0usize;
    for s in &sessions {
        if !s.is_running() {
            let _ = s.delete();
            count += 1;
        }
    }
    count
}
