mod lsp;
mod parser;

#[tokio::main]
async fn main() {
    if std::env::args()
        .map(|s| s.to_lowercase())
        .any(|arg| arg == "-v" || arg == "--version")
    {
        println!("color-lsp v{}", env!("CARGO_PKG_VERSION"));
        return;
    } else if std::env::args().len() > 0 {
        println!("Usage: color-lsp [options]");
        println!("Options:");
        println!("  -v, --version    Print version information");
        println!("  -h, --help       Print this help message");
        return;
    }

    lsp::start().await;
}
