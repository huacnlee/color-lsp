mod lsp;
mod parser;

#[tokio::main]
async fn main() {
    lsp::start().await;
}
