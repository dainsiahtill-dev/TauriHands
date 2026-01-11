use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct WorkspaceState {
    root: Arc<Mutex<PathBuf>>,
}

impl WorkspaceState {
    pub fn new(root: PathBuf) -> Self {
        let canonical = canonicalize_or(root);
        Self {
            root: Arc::new(Mutex::new(canonical)),
        }
    }

    pub fn root(&self) -> PathBuf {
        self.root.lock().expect("workspace lock poisoned").clone()
    }

    pub fn set_root(&self, input: &str) -> Result<PathBuf, String> {
        let path = normalize_root_input(input);
        if !path.exists() {
            return Err(format!("Workspace root not found: {}", path.display()));
        }
        if !path.is_dir() {
            return Err("Workspace root must be a directory".to_string());
        }
        let canonical = path.canonicalize().unwrap_or(path);
        *self.root.lock().expect("workspace lock poisoned") = canonical.clone();
        Ok(canonical)
    }

    pub fn resolve_path(&self, input: &str) -> Result<PathBuf, String> {
        let root = self.root();
        let candidate = resolve_candidate(&root, input);
        let canonical = candidate
            .canonicalize()
            .map_err(|e| format!("Path not found: {}", e))?;
        ensure_within_root(&root, &canonical)?;
        Ok(canonical)
    }

    pub fn resolve_path_for_write(&self, input: &str) -> Result<PathBuf, String> {
        let root = self.root();
        let candidate = resolve_candidate(&root, input);
        let parent = candidate
            .parent()
            .ok_or_else(|| "Invalid file path".to_string())?;
        let canonical_parent = parent
            .canonicalize()
            .map_err(|e| format!("Invalid parent directory: {}", e))?;
        ensure_within_root(&root, &canonical_parent)?;
        Ok(candidate)
    }
}

pub fn default_workspace_root() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.file_name().and_then(|name| name.to_str()) == Some("src-tauri") {
        cwd.parent().map(|p| p.to_path_buf()).unwrap_or(cwd)
    } else {
        cwd
    }
}

pub fn display_path(path: &Path) -> String {
    let raw = path.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        strip_windows_namespace_prefix(&raw)
    }
    #[cfg(not(windows))]
    {
        raw
    }
}

fn resolve_candidate(root: &Path, input: &str) -> PathBuf {
    let normalized = normalize_path_input(input);
    let path = Path::new(&normalized);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn ensure_within_root(root: &Path, candidate: &Path) -> Result<(), String> {
    let canonical_root = canonicalize_or(root.to_path_buf());
    if candidate.starts_with(&canonical_root) {
        Ok(())
    } else {
        Err("Path escapes workspace root".to_string())
    }
}

fn canonicalize_or(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn normalize_root_input(input: &str) -> PathBuf {
    PathBuf::from(normalize_path_input(input))
}

fn normalize_path_input(input: &str) -> String {
    let trimmed = input.trim();
    let unquoted = strip_wrapping_quotes(trimmed);
    #[cfg(windows)]
    let normalized = strip_windows_namespace_prefix(unquoted);
    #[cfg(not(windows))]
    let normalized = unquoted.to_string();
    normalized
}

fn strip_wrapping_quotes(input: &str) -> &str {
    if input.len() < 2 {
        return input;
    }
    let bytes = input.as_bytes();
    let first = bytes[0];
    let last = bytes[input.len() - 1];
    if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
        &input[1..input.len() - 1]
    } else {
        input
    }
}

#[cfg(windows)]
fn strip_windows_namespace_prefix(input: &str) -> String {
    if let Some(stripped) = input.strip_prefix(r"\\?\") {
        if let Some(unc) = stripped.strip_prefix("UNC\\") {
            format!(r"\\{}", unc)
        } else {
            stripped.to_string()
        }
    } else {
        input.to_string()
    }
}
