use pwmgr_core::paths::AppPaths;
use pwmgr_core::session::SystemClock;
use pwmgr_web::{build_router, AppContext, SessionStore, PORT};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let home = dirs::home_dir().expect("could not resolve home directory");
    let paths = AppPaths::new(home.join(".pwmgr"));
    paths.ensure_app_dirs().expect("failed to create ~/.pwmgr directories");

    let ctx = AppContext { store: SessionStore::new(), paths: Arc::new(paths) };

    // Idle sweep: every 2s (same cadence as the desktop autolock::tick loop), lock idle sessions and
    // prune entries idle beyond 24h. Runs for the life of the process.
    {
        let store = ctx.store.clone();
        tokio::spawn(async move {
            let clock = SystemClock;
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                store.sweep(&clock);
            }
        });
    }

    let app = build_router(ctx);
    // 127.0.0.1 ONLY - the primary network-exposure mitigation (THREAT_MODEL A4). Never 0.0.0.0.
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", PORT))
        .await
        .expect("failed to bind 127.0.0.1");
    println!("pwmgr-web listening on http://127.0.0.1:{PORT} (serving ./dist)");
    axum::serve(listener, app).await.expect("server error");
}
