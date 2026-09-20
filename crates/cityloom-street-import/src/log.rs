//! The local-only failure log (`security-design.md` SD-1, SD-2): the
//! underlying dependency error or panic message is captured **once**,
//! locally, and never transmitted anywhere or returned to a caller.

/// Logs `detail` locally against `street_name`, never returning it and
/// never sending it anywhere (SD-2). `eprintln!` is a deliberately simple
/// choice — this crate carries no logging-framework dependency of its own,
/// and stderr satisfies "local only, never transmitted" on every target
/// this crate builds for, including `wasm32-unknown-unknown` under a
/// WASI-less browser host (where it is inert rather than harmful).
pub(crate) fn log_locally(street_name: &str, detail: &str) {
    eprintln!("cityloom-street-import: {street_name}: {detail}");
}
