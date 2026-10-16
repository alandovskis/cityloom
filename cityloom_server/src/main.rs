use base64::Engine;
use cityloom_server::config::{AccountSide, Config};
use zeroize::Zeroizing;

fn fail(code: i32, message: impl std::fmt::Display) -> ! {
    eprintln!("{message}");
    std::process::exit(code)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();
    let config = Config::from_lookup(&|k| std::env::var(k).ok()).unwrap_or_else(|e| fail(2, e));
    let Some(command) = std::env::args().nth(1) else {
        if let Err(e) = cityloom_server::run(config).await {
            fail(1, format!("the server stopped: {e}"));
        }
        return;
    };
    // `cityloom_server rewrap` or `cityloom_server rehash`: they need the account side, and so its variables.
    let account = match &config.account {
        AccountSide::On(account) => account,
        AccountSide::Off(reason) => fail(2, reason),
    };
    // The lookup key the data was hashed with, for `rehash`; `LOOKUP_KEY` is the new one.
    let old_lookup_key = match std::env::var("OLD_LOOKUP_KEY") {
        Err(_) => None,
        Ok(text) => {
            let bytes = Zeroizing::new(base64::engine::general_purpose::STANDARD.decode(text.trim()).unwrap_or_default());
            let Ok(key) = <[u8; 32]>::try_from(&bytes[..]) else { fail(2, "OLD_LOOKUP_KEY is not valid") };
            Some(Zeroizing::new(key))
        }
    };
    match cityloom_server::commands::run(&command, account, old_lookup_key).await {
        Ok(done) => println!("{done}"),
        Err(e) => fail(1, e),
    }
}
