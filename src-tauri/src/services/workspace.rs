use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
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
        if candidate.exists() {
            let canonical = candidate
                .canonicalize()
                .map_err(|e| format!("Invalid file path: {}", e))?;
            ensure_within_root(&root, &canonical)?;
            return Ok(candidate);
        }
        let canonical_root = canonicalize_or(root.clone());
        let normalized = lexical_normalize(&candidate);
        ensure_within_root_lexical(&canonical_root, &normalized)?;
        Ok(normalized)
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

const READ_FALLBACK_EXTS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "vue", "mjs", "cjs", "mts", "cts", "json", "md", "toml", "yaml",
    "yml",
];

pub fn resolve_read_path_with_fallback(
    workspace: &WorkspaceState,
    input: &str,
) -> Result<PathBuf, String> {
    let normalized = normalize_path_input(input);
    let candidates = build_read_candidates(&normalized);
    let mut last_error = None;

    for candidate in candidates {
        match workspace.resolve_path(&candidate) {
            Ok(resolved) => {
                if resolved.is_file() {
                    return Ok(resolved);
                }
                if resolved.is_dir() {
                    if let Some(found) = find_index_file(&resolved) {
                        return Ok(found);
                    }
                    last_error = Some("Path is a directory".to_string());
                } else {
                    last_error = Some("Path is not a file".to_string());
                }
            }
            Err(err) => last_error = Some(err),
        }
    }

    if let Some(found) = resolve_by_stem(workspace, &normalized) {
        return Ok(found);
    }

    Err(last_error.unwrap_or_else(|| "Path not found".to_string()))
}

fn build_read_candidates(input: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    let mut push_candidate = |value: String| {
        if seen.insert(value.clone()) {
            candidates.push(value);
        }
    };

    push_candidate(input.to_string());

    let path = Path::new(input);
    if path.file_name().is_none() {
        return candidates;
    }

    if let Some(ext) = path.extension().and_then(|value| value.to_str()) {
        for alt in READ_FALLBACK_EXTS {
            if ext.eq_ignore_ascii_case(alt) {
                continue;
            }
            let mut alt_path = path.to_path_buf();
            alt_path.set_extension(alt);
            push_candidate(alt_path.to_string_lossy().to_string());
        }
    } else {
        for alt in READ_FALLBACK_EXTS {
            let mut alt_path = path.to_path_buf();
            alt_path.set_extension(alt);
            push_candidate(alt_path.to_string_lossy().to_string());
        }
    }

    candidates
}

fn find_index_file(dir: &Path) -> Option<PathBuf> {
    for ext in READ_FALLBACK_EXTS {
        let candidate = dir.join(format!("index.{}", ext));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn resolve_by_stem(workspace: &WorkspaceState, input: &str) -> Option<PathBuf> {
    let path = Path::new(input);
    let stem = path.file_stem()?.to_string_lossy().to_string();
    let parent = path.parent();
    let parent_resolved = match parent {
        None => workspace.root(),
        Some(value) if value.as_os_str().is_empty() => workspace.root(),
        Some(value) => workspace.resolve_path(&value.to_string_lossy()).ok()?,
    };

    if !parent_resolved.is_dir() {
        return None;
    }

    let mut matches: Vec<PathBuf> = Vec::new();
    let entries = fs::read_dir(parent_resolved).ok()?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }
        let entry_stem = entry_path.file_stem().and_then(|value| value.to_str());
        if entry_stem
            .map(|value| value.eq_ignore_ascii_case(&stem))
            .unwrap_or(false)
        {
            matches.push(entry_path);
        }
    }

    if matches.is_empty() {
        return None;
    }

    matches.sort_by_key(|path| {
        path.extension()
            .and_then(|value| value.to_str())
            .and_then(|ext| {
                READ_FALLBACK_EXTS
                    .iter()
                    .position(|candidate| ext.eq_ignore_ascii_case(candidate))
            })
            .unwrap_or(usize::MAX)
    });

    matches.into_iter().next()
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

fn ensure_within_root_lexical(root: &Path, candidate: &Path) -> Result<(), String> {
    let root_compare = normalize_path_for_compare(root);
    let candidate_compare = normalize_path_for_compare(candidate);
    if candidate_compare.starts_with(&root_compare) {
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

fn normalize_path_for_compare(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy().to_string();
    #[cfg(windows)]
    let normalized = strip_windows_namespace_prefix(&raw);
    #[cfg(not(windows))]
    let normalized = raw;
    lexical_normalize(Path::new(&normalized))
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut prefix: Option<std::ffi::OsString> = None;
    let mut has_root = false;
    let mut parts: Vec<std::ffi::OsString> = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix_component) => {
                prefix = Some(prefix_component.as_os_str().to_owned());
            }
            Component::RootDir => {
                has_root = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !parts.is_empty() {
                    parts.pop();
                }
            }
            Component::Normal(value) => {
                parts.push(value.to_owned());
            }
        }
    }

    let mut result = PathBuf::new();
    if let Some(prefix) = prefix {
        result.push(prefix);
    }
    if has_root {
        result.push(Path::new(std::path::MAIN_SEPARATOR_STR));
    }
    for part in parts {
        result.push(part);
    }
    result
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
