use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{self, *};
use tower_lsp::{Client, LanguageServer, LspService, Server};

const LSP_NAME: &str = "ColorLSP";

struct Backend {
    client: Client,
    work_dir: RwLock<PathBuf>,
    documents: RwLock<HashMap<Url, Arc<TextDocumentItem>>>,
    diagnostics: RwLock<HashMap<Url, Vec<Diagnostic>>>,
    colors: RwLock<HashMap<Url, Vec<ColorInformation>>>,
}

#[allow(unused)]
impl Backend {
    fn work_dir(&self) -> PathBuf {
        self.work_dir.read().unwrap().clone()
    }

    fn set_work_dir(&self, work_dir: PathBuf) {
        *self.work_dir.write().unwrap() = work_dir;
    }

    fn upsert_document(&self, doc: Arc<TextDocumentItem>) {
        let uri = doc.uri.clone();
        self.documents
            .write()
            .unwrap()
            .get_mut(&uri)
            .map(|old| std::mem::replace(old, doc.clone()));
    }

    fn get_document(&self, uri: &Url) -> Option<Arc<TextDocumentItem>> {
        self.documents.read().unwrap().get(uri).cloned()
    }

    fn remove_document(&self, uri: &Url) {
        self.documents.write().unwrap().remove(uri);
        self.colors.write().unwrap().remove(uri);
        self.diagnostics.write().unwrap().remove(uri);
    }

    async fn send_diagnostics(&self, document: &TextDocumentItem, diagnostics: Vec<Diagnostic>) {
        if let Ok(mut map) = self.diagnostics.write() {
            map.entry(document.uri.clone())
                .and_modify(|old_diagnostics| old_diagnostics.extend_from_slice(&diagnostics))
                .or_insert_with(|| diagnostics.clone());
        }
        self.client
            .publish_diagnostics(document.uri.clone(), diagnostics, None)
            .await;
    }

    async fn clear_diagnostics(&self, uri: &Url) {
        self.diagnostics.write().unwrap().remove(uri);
        self.client
            .publish_diagnostics(uri.clone(), vec![], None)
            .await;
    }

    async fn clear_all_diagnostic(&self) {
        let uris = self
            .documents
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        for uri in uris.iter() {
            self.clear_diagnostics(uri).await;
        }
    }

    /// Scan all colors vars in the document
    async fn scan_document(&self, document: &TextDocumentItem) {
        let input = document.text.as_str();
        let nodes = crate::parser::parse(input);
        let mut colors = vec![];
        for node in nodes.iter() {
            let info = ColorInformation {
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: node.loc.0.saturating_sub(1) as u32,
                        character: node.loc.1.saturating_sub(1) as u32,
                    },
                    end: lsp_types::Position {
                        line: node.loc.0.saturating_sub(1) as u32,
                        character: (node.loc.1.saturating_sub(1) + node.matched.len()) as u32,
                    },
                },
                color: node.lsp_color(),
            };
            colors.push(info);
        }

        if let Ok(mut map) = self.colors.write() {
            map.insert(document.uri.clone(), colors);
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root_uri) = params.root_uri {
            let root_path = root_uri.to_file_path().unwrap();
            self.set_work_dir(root_path.clone());
        }

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: LSP_NAME.into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        ..Default::default()
                    },
                )),
                document_formatting_provider: Some(OneOf::Left(false)),
                color_provider: Some(ColorProviderCapability::Simple(true)),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![
                            CodeActionKind::QUICKFIX,
                            CodeActionKind::SOURCE_FIX_ALL,
                        ]),
                        ..Default::default()
                    },
                )),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let DidOpenTextDocumentParams { text_document } = params;
        self.upsert_document(Arc::new(text_document.clone()));
        self.scan_document(&text_document).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let DidCloseTextDocumentParams { text_document } = params;
        self.remove_document(&text_document.uri);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let DidChangeTextDocumentParams {
            text_document,
            content_changes,
        } = params;
        let VersionedTextDocumentIdentifier { uri, version } = text_document;

        assert_eq!(content_changes.len(), 1);
        let change = content_changes.into_iter().next().unwrap();
        assert!(change.range.is_none());

        let updated_doc =
            TextDocumentItem::new(uri.clone(), "".to_string(), version, change.text.clone());

        self.upsert_document(Arc::new(updated_doc.clone()));
        self.scan_document(&updated_doc).await;
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {}

    async fn formatting(&self, _: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        Ok(None)
    }

    async fn code_action(&self, _: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        return Ok(None);
    }

    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        // self.client
        //     .log_message(
        //         MessageType::INFO,
        //         format!("-- document_color: {}", params.text_document.uri),
        //     )
        //     .await;

        let colors = self
            .colors
            .read()
            .unwrap()
            .get(&params.text_document.uri)
            .cloned()
            .unwrap_or_default();

        // self.client
        //     .log_message(MessageType::INFO, format!("document_color {:?}\n", colors))
        //     .await;

        Ok(colors)
    }
}

pub async fn start() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        work_dir: RwLock::new(PathBuf::new()),
        documents: RwLock::new(HashMap::new()),
        diagnostics: RwLock::new(HashMap::new()),
        colors: RwLock::new(HashMap::new()),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
