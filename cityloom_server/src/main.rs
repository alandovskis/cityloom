#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();
    let config = match cityloom_server::config::Config::from_lookup(&|k| std::env::var(k).ok()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = cityloom_server::run(config).await {
        eprintln!("the server stopped: {e}");
        std::process::exit(1);
    }
}
