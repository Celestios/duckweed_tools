//! Entry point: parse args, start server.

use std::env;
use std::path::PathBuf;

use clap::Parser;
use duckweed_server::data::store::AppState;
use duckweed_server::server::create_router;

#[derive(Parser)]
#[command(name = "duckweed-server", version, about = "Duckweed & Cultivation Toolkit — single-binary web server")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value = "8000")]
    port: u16,

    /// Data directory (default: same directory as binary)
    #[arg(short, long)]
    data_dir: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Resolve data directory: default to binary's directory
    let data_dir = match cli.data_dir {
        Some(dir) => PathBuf::from(dir),
        None => {
            env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
        }
    };

    let state = AppState::new(data_dir);
    let app = create_router(state);

    let addr = format!("0.0.0.0:{}", cli.port);
    println!();
    println!("  Duckweed Cultivation Toolkit — Rust Server");
    println!("  Open in browser: http://localhost:{}", cli.port);
    println!();

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");
    axum::serve(listener, app)
        .await
        .expect("Server error");
}
