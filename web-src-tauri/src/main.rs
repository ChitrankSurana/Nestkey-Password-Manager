use pwmgr_core::paths::AppPaths;
use pwmgr_core::session::SystemClock;
use pwmgr_web::{build_router, AppContext, KdfGuard, SessionStore, ThrottleStore, PORT};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let home = dirs::home_dir().expect("could not resolve home directory");
    let paths = AppPaths::new(home.join(".pwmgr"));
    paths.ensure_app_dirs().expect("failed to create ~/.pwmgr directories");

    let ctx = AppContext {
        store: SessionStore::new(),
        paths: Arc::new(paths),
        throttle: ThrottleStore::new(),
        kdf: KdfGuard::new(),
    };

    // Idle sweep: every 2s (same cadence as the desktop autolock::tick loop), lock idle sessions and
    // prune entries idle beyond 24h. Runs for the life of the process.
    {
        let store = ctx.store.clone();
        let throttle = ctx.throttle.clone();
        tokio::spawn(async move {
            let clock = SystemClock;
            let tick_interval = Duration::from_secs(2);
            let mut last_tick = std::time::Instant::now();
            loop {
                tokio::time::sleep(tick_interval).await;
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(last_tick);
                last_tick = now;
                // The host machine (not just one session) appears to have been suspended - force
                // every live session locked immediately rather than waiting on each one's own
                // idle timeout to eventually catch up.
                if pwmgr_core::autolock::suspend_gap_exceeds(elapsed, tick_interval) {
                    store.lock_all();
                }
                store.sweep(&clock);
                throttle.sweep(&clock);
            }
        });
    }

    let app = build_router(ctx);
    // 127.0.0.1 ONLY - the primary network-exposure mitigation (THREAT_MODEL A4). Never 0.0.0.0.
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", PORT))
        .await
        .expect("failed to bind 127.0.0.1");
    println!("pwmgr-web listening on http://127.0.0.1:{PORT} (serving ./dist)");
    // Connection info feeds the IP throttle key (SEC-02/D-09).
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .await
        .expect("server error");
}
