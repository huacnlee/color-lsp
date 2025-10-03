use std::fs;
use zed_extension_api::{self as zed, Result};

const GITHUB_REPO: &str = "huacnlee/color-lsp";
const BIN_NAME: &str = "color-lsp";

struct ColorHighlightExtension {
    cached_binary_path: Option<String>,
}

enum Status {
    None,
    CheckingForUpdate,
    Downloading,
    Failed(String),
}

fn update_status(id: &zed::LanguageServerId, status: Status) {
    match status {
        Status::None => zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::None,
        ),
        Status::CheckingForUpdate => zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        ),
        Status::Downloading => zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::Downloading,
        ),
        Status::Failed(msg) => zed::set_language_server_installation_status(
            id,
            &zed::LanguageServerInstallationStatus::Failed(msg),
        ),
    }
}

impl ColorHighlightExtension {
    fn language_server_binary_path(
        &mut self,
        id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        // Check if the binary is already installed by manually checking the path
        if let Some(path) = worktree.which(BIN_NAME) {
            return Ok(path);
        }

        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                update_status(id, Status::None);
                return Ok(path.clone());
            }
        }

        let binary_path = format!("color-lsp-latest/{BIN_NAME}");
        let has_binary =
            fs::metadata(&binary_path).map_or(false, |stat| stat.is_file() || stat.is_symlink());

        if has_binary {
            // silent to check for update.
            let _ = Self::check_to_update(&binary_path, &id);
            return Ok(binary_path);
        }

        let version_binary_path = Self::check_to_update(&binary_path, id)?;
        self.cached_binary_path = Some(version_binary_path.clone());
        Ok(version_binary_path)
    }

    fn check_to_update(binary_path: &str, id: &zed::LanguageServerId) -> Result<String> {
        let (platform, arch) = zed::current_platform();
        update_status(id, Status::CheckingForUpdate);

        let release = zed::latest_github_release(
            GITHUB_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let asset_name = format!(
            "color-lsp-{os}-{arch}.{ext}",
            arch = match arch {
                zed::Architecture::Aarch64 => "arm64",
                zed::Architecture::X86 => "amd64",
                zed::Architecture::X8664 => "amd64",
            },
            os = match platform {
                zed::Os::Mac => "darwin",
                zed::Os::Linux => "linux",
                zed::Os::Windows => "windows",
            },
            ext = match platform {
                zed::Os::Windows => "zip",
                _ => "tar.gz",
            }
        );

        let file_type = match platform {
            zed::Os::Windows => zed::DownloadedFileType::Zip,
            _ => zed::DownloadedFileType::GzipTar,
        };

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = binary_path
            .split_once('/')
            .map(|s| s.0)
            .unwrap_or("color-lsp-latest");

        if !fs::metadata(&binary_path).map_or(false, |stat| stat.is_file()) {
            update_status(id, Status::Downloading);
            zed::download_file(&asset.download_url, &version_dir, file_type)
                .map_err(|e| format!("failed to download file: {e}"))?;

            let entries =
                fs::read_dir(".").map_err(|e| format!("failed to list working directory {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("failed to load directory entry {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
            update_status(id, Status::None);
        }

        Ok(binary_path.to_string())
    }
}

impl zed::Extension for ColorHighlightExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let command = self
            .language_server_binary_path(id, worktree)
            .inspect_err(|err| {
                update_status(id, Status::Failed(err.to_string()));
            })?;

        Ok(zed::Command {
            command,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(ColorHighlightExtension);
