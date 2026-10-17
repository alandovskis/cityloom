//! Run by `tests/progress.rs`: one step that blocks, with Ctrl-C handled as the prepare command handles it.

use std::sync::Arc;

use cityloom_prepare::progress::{Progress, Spec, exit_on_ctrl_c};

fn main() {
    let dir = std::env::temp_dir().join(format!("cityloom-ctrl-c-{}", std::process::id()));
    let spec = Spec { key: "a", group: "g", subject: "first".into() };
    let out = Box::new(std::io::stdout());
    let clock = Arc::new(|| 0.0);
    let progress = Arc::new(Progress::with(vec![spec], dir.join("logs"), dir, out, false, clock));
    exit_on_ctrl_c(&progress).unwrap();
    let _ = progress.step("a", |_| {
        std::thread::sleep(std::time::Duration::from_secs(60));
        Ok(())
    });
}
