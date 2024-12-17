use fuji_engine::*;
use tracing_subscriber::fmt;

fn main() {
    fmt::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_target(true)
        .init();

    if let Err(e) = run() {
        tracing::error!("Error: {:?}", e);
    }
}
