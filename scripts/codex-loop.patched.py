import argparse
import datetime
import json
import os
import re
import shutil
import subprocess
import sys
import threading
import time
from typing import Any, Dict, List, Optional

PORTS = [1420, 1421]  # Tauri/Vite dev server + HMR
ANSI_RESET = "\x1b[0m"
ANSI_COLORS = {
    "INFO": "\x1b[36m",
    "TURN": "\x1b[34m",
    "COMMAND": "\x1b[33m",
    "FILE": "\x1b[32m",
    "TOOL": "\x1b[35m",
    "THINKING": "\x1b[90m",
    "ERROR": "\x1b[31m",
    "AGENT": "\x1b[36m",
}
ANSI_ESCAPE_RE = re.compile(r"\x1b\[[0-9;]*m")
ANSI_ENABLED = False
RATE_LIMIT_SECONDS_RE = re.compile(r'resets_in_seconds"?\s*:\s*(\d+)', re.IGNORECASE)
RATE_LIMIT_EPOCH_RE   = re.compile(r'resets_at"?\s*:\s*(\d+)', re.IGNORECASE)
RATE_LIMIT_RETRY_AFTER_RE = re.compile(r'retry[-_ ]after"?\s*:\s*(\d+)', re.IGNORECASE)
IGNORABLE_ERROR_PATTERNS = [
    r"rmcp::transport::worker",
    r"AuthRequired\(AuthRequiredError",
    r"invalid_token",
    r"OAuth token exchange failed",
    r"mcp\.notion\.com/mcp",
    r"mcp\.linear\.app/mcp",
    r"unexpected EOF during handshake",
]


def supports_color() -> bool:
    if not ANSI_ENABLED:
        return False
    if os.environ.get("NO_COLOR"):
        return False
    return sys.stdout.isatty()


def colorize(label: str, text: str, enabled: bool) -> str:
    if not enabled:
        return f"[{label}] {text}"
    color = ANSI_COLORS.get(label, "")
    if color:
        return f"{color}[{label}] {text}{ANSI_RESET}"
    return f"[{label}] {text}"


def safe_truncate(text: str, limit: int = 200) -> str:
    if len(text) <= limit:
        return text
    return text[:limit] + "..."


def strip_ansi(text: str) -> str:
    if not text:
        return text
    return ANSI_ESCAPE_RE.sub("", text)


def extract_rate_limit_seconds(text: str) -> int:
    if not text:
        return 0
    match = RATE_LIMIT_RETRY_AFTER_RE.search(text)
    if match:
        try:
            return max(0, int(match.group(1)))
        except ValueError:
            return 0
    match = RATE_LIMIT_SECONDS_RE.search(text)
    if match:
        try:
            return max(0, int(match.group(1)))
        except ValueError:
            return 0
    match = RATE_LIMIT_EPOCH_RE.search(text)
    if match:
        try:
            reset_at = int(match.group(1))
            now = int(time.time())
            return max(0, reset_at - now)
        except ValueError:
            return 0
    return 0


def is_ignorable_error_line(text: str) -> bool:
    if not text:
        return False
    for pattern in IGNORABLE_ERROR_PATTERNS:
        try:
            if re.search(pattern, text, flags=re.IGNORECASE):
                return True
        except re.error:
            continue
    return False


def unique_preserve(items: List[str]) -> List[str]:
    seen = set()
    output = []
    for item in items:
        if item in seen:
            continue
        seen.add(item)
        output.append(item)
    return output


def normalize_path(text: str) -> str:
    if not text:
        return ""
    path = text.strip().strip("'\"")
    path = path.rstrip(").,;")
    return path


def extract_text_from_content(content: Any) -> str:
    if isinstance(content, str):
        return content.strip()
    if isinstance(content, dict):
        if "text" in content and isinstance(content["text"], str):
            return content["text"].strip()
        if "content" in content and isinstance(content["content"], str):
            return content["content"].strip()
    if isinstance(content, list):
        parts = []
        for item in content:
            if isinstance(item, dict):
                if item.get("type") in ("text", "output_text", "input_text"):
                    text = item.get("text") or item.get("content")
                    if isinstance(text, str):
                        parts.append(text.strip())
        return " ".join(part for part in parts if part)
    return ""


def append_log_raw(log_path: str, text: str) -> None:
    if not log_path or not text:
        return
    try:
        with open(log_path, "a", encoding="utf-8") as handle:
            handle.write(text)
    except Exception:
        pass


def get_event_type(event: Dict[str, Any]) -> str:
    value = (
        event.get("type")
        or event.get("event")
        or event.get("kind")
        or event.get("event_type")
        or event.get("name")
        or ""
    )
    return str(value).lower()


def extract_stream_text(event: Dict[str, Any]) -> str:
    event_type = get_event_type(event)
    if not event_type:
        event_type = ""

    def text_from_obj(obj: Any) -> str:
        if isinstance(obj, str) and obj.strip():
            return obj
        if isinstance(obj, dict):
            if isinstance(obj.get("text"), str) and obj.get("text").strip():
                return obj.get("text").strip()
            return extract_text_from_content(obj.get("content"))
        return ""

    # Common delta formats (Responses streaming)
    if "output_text" in event_type or "assistant" in event_type or "message" in event_type:
        delta = event.get("delta")
        text = text_from_obj(delta)
        if text:
            return text
        text = text_from_obj(event.get("text"))
        if text:
            return text

    item = event.get("item")
    if isinstance(item, dict):
        itype = str(item.get("type") or "").lower()
        if itype in ("output_text", "output_text_delta", "assistant_message", "agent_message"):
            text = text_from_obj(item.get("text"))
            if text:
                return text
            text = text_from_obj(item.get("content"))
            if text:
                return text

    return ""


def is_output_done_event(event: Dict[str, Any]) -> bool:
    event_type = get_event_type(event)
    if "output_text.done" in event_type:
        return True
    if event_type.endswith(".done") and ("output_text" in event_type or "message" in event_type):
        return True
    item = event.get("item")
    if isinstance(item, dict):
        itype = str(item.get("type") or "").lower()
        if itype in ("output_text_done", "assistant_message", "agent_message"):
            return True
    return False


def is_reasoning_event(event_type: str, event: Dict[str, Any]) -> bool:
    lowered = event_type.lower()
    if "reason" in lowered or "analysis" in lowered or "thinking" in lowered:
        return True
    item = event.get("item")
    if isinstance(item, dict):
        if item.get("type") in ("reasoning", "analysis", "thinking"):
            return True
        content = item.get("content")
        if isinstance(content, list):
            for entry in content:
                if isinstance(entry, dict) and entry.get("type") in ("reasoning", "analysis", "thinking"):
                    return True
    return False


def categorize_event(event_type: str, event: Dict[str, Any]) -> str:
    item = event.get("item")
    if isinstance(item, dict):
        item_type = str(item.get("type") or "").lower()
        if item_type in ("command_execution", "command", "shell"):
            return "COMMAND"
        if item_type in ("file_write", "file", "patch", "file_change", "file_edit"):
            return "FILE"
        if item_type in ("tool_call", "tool_result", "function_call", "function"):
            return "TOOL"
        if item_type in ("agent_message", "assistant_message"):
            return "AGENT"
        if item_type in ("reasoning", "analysis", "thinking"):
            return "THINKING"
    lowered = event_type.lower()
    if "error" in lowered or event.get("error") or event.get("exception"):
        return "ERROR"
    if is_reasoning_event(event_type, event):
        return "THINKING"
    if "tool" in lowered or "function" in lowered:
        return "TOOL"
    if "command" in lowered or "shell" in lowered or "process" in lowered:
        return "COMMAND"
    if "file" in lowered or "patch" in lowered:
        return "FILE"
    if "turn" in lowered or "thread" in lowered:
        return "TURN"
    return "INFO"


def summarize_event(event: Dict[str, Any]) -> str:
    if not isinstance(event, dict):
        return safe_truncate(str(event))
    for key in ("summary", "message", "status", "name"):
        value = event.get(key)
        if isinstance(value, str) and value.strip():
            return safe_truncate(value.strip())
    data = event.get("data")
    if isinstance(data, dict):
        for key in ("summary", "message", "status", "name", "command", "path", "file"):
            value = data.get(key)
            if isinstance(value, str) and value.strip():
                return safe_truncate(value.strip())
    item = event.get("item")
    if isinstance(item, dict):
        text_value = item.get("text")
        if isinstance(text_value, str) and text_value.strip():
            return safe_truncate(text_value.strip())
        text = extract_text_from_content(item.get("content"))
        if text:
            return safe_truncate(text)
        if isinstance(item.get("type"), str):
            return safe_truncate(item["type"])
    return ""

def extract_command_detail(event: Dict[str, Any]) -> str:
    if not isinstance(event, dict):
        return ""
    candidates = [
        event.get("command"),
        event.get("cmd"),
        event.get("argv"),
        event.get("args"),
    ]
    data = event.get("data")
    if isinstance(data, dict):
        candidates.extend([data.get("command"), data.get("cmd"), data.get("argv"), data.get("args")])
        request = data.get("request")
        if isinstance(request, dict):
            candidates.extend([request.get("command"), request.get("cmd"), request.get("argv"), request.get("args")])
    item = event.get("item")
    if isinstance(item, dict):
        candidates.extend([item.get("command"), item.get("cmd"), item.get("argv"), item.get("args")])
        content = item.get("content")
        if isinstance(content, list):
            for entry in content:
                if isinstance(entry, dict) and entry.get("type") in ("command", "command_execution"):
                    text = entry.get("text") or entry.get("content")
                    if isinstance(text, str) and text.strip():
                        candidates.append(text.strip())
    for cand in candidates:
        if isinstance(cand, str) and cand.strip():
            return safe_truncate(cand.strip(), 240)
        if isinstance(cand, list):
            parts = [str(part) for part in cand if str(part).strip()]
            if parts:
                return safe_truncate(" ".join(parts), 240)
    return ""

def extract_command_status(event: Dict[str, Any]) -> str:
    if not isinstance(event, dict):
        return ""
    status = ""
    exit_code = None
    item = event.get("item")
    if isinstance(item, dict):
        if isinstance(item.get("status"), str):
            status = item["status"].strip()
        if item.get("exit_code") is not None:
            exit_code = item.get("exit_code")
    data = event.get("data")
    if isinstance(data, dict):
        if not status and isinstance(data.get("status"), str):
            status = data["status"].strip()
        if exit_code is None and data.get("exit_code") is not None:
            exit_code = data.get("exit_code")
    parts = []
    if status:
        parts.append(status)
    if exit_code is not None:
        parts.append(f"exit={exit_code}")
    return " ".join(parts).strip()

def extract_reasoning_summary(event: Dict[str, Any]) -> str:
    if not isinstance(event, dict):
        return ""
    for key in ("summary", "reasoning_summary"):
        value = event.get(key)
        if isinstance(value, str) and value.strip():
            return safe_truncate(value.strip())
    item = event.get("item")
    if isinstance(item, dict):
        text_value = item.get("text")
        if isinstance(text_value, str) and text_value.strip():
            return safe_truncate(text_value.strip(), 240)
        for key in ("summary", "reasoning_summary"):
            value = item.get(key)
            if isinstance(value, str) and value.strip():
                return safe_truncate(value.strip())
        content = item.get("content")
        if isinstance(content, list):
            for entry in content:
                if isinstance(entry, dict):
                    if entry.get("type") in ("summary", "reasoning_summary"):
                        text = entry.get("text") or entry.get("content")
                        if isinstance(text, str) and text.strip():
                            return safe_truncate(text.strip())
    data = event.get("data")
    if isinstance(data, dict):
        for key in ("summary", "reasoning_summary"):
            value = data.get(key)
            if isinstance(value, str) and value.strip():
                return safe_truncate(value.strip())
    return ""


def extract_paths_from_command(command: str) -> Dict[str, List[str]]:
    if not command:
        return {"read": [], "write": []}
    reads: List[str] = []
    writes: List[str] = []
    read_patterns = [
        r"Get-Content\b[^\n]*?(?:-Path\s+)?(?P<path>'[^']+'|\"[^\"]+\"|\S+)",
    ]
    write_patterns = [
        r"Set-Content\b[^\n]*?(?:-Path\s+)?(?P<path>'[^']+'|\"[^\"]+\"|\S+)",
        r"Add-Content\b[^\n]*?(?:-Path\s+)?(?P<path>'[^']+'|\"[^\"]+\"|\S+)",
        r"Out-File\b[^\n]*?(?:-FilePath\s+|-Path\s+)?(?P<path>'[^']+'|\"[^\"]+\"|\S+)",
        r"New-Item\b[^\n]*?(?:-Path\s+)?(?P<path>'[^']+'|\"[^\"]+\"|\S+)",
        r"Remove-Item\b[^\n]*?(?:-Path\s+)?(?P<path>'[^']+'|\"[^\"]+\"|\S+)",
    ]
    for pattern in read_patterns:
        for match in re.finditer(pattern, command, flags=re.IGNORECASE):
            path = normalize_path(match.group("path"))
            if path and not path.startswith("-"):
                reads.append(path)
    for pattern in write_patterns:
        for match in re.finditer(pattern, command, flags=re.IGNORECASE):
            path = normalize_path(match.group("path"))
            if path and not path.startswith("-"):
                writes.append(path)
    return {"read": unique_preserve(reads), "write": unique_preserve(writes)}


def extract_paths_from_event(event: Dict[str, Any]) -> List[str]:
    paths: List[str] = []
    if not isinstance(event, dict):
        return paths
    for key in ("path", "file", "filename"):
        value = event.get(key)
        if isinstance(value, str) and value.strip():
            paths.append(normalize_path(value))
    data = event.get("data")
    if isinstance(data, dict):
        for key in ("path", "file", "filename"):
            value = data.get(key)
            if isinstance(value, str) and value.strip():
                paths.append(normalize_path(value))
    item = event.get("item")
    if isinstance(item, dict):
        for key in ("path", "file", "filename"):
            value = item.get(key)
            if isinstance(value, str) and value.strip():
                paths.append(normalize_path(value))
        content = item.get("content")
        if isinstance(content, list):
            for entry in content:
                if isinstance(entry, dict):
                    value = entry.get("path") or entry.get("file")
                    if isinstance(value, str) and value.strip():
                        paths.append(normalize_path(value))
    return unique_preserve([p for p in paths if p])


def summarize_run_events(
    events: Optional[List[Dict[str, Any]]],
    duration_seconds: float,
    json_enabled: bool,
    max_items: int = 20,
) -> str:
    lines = ["### Run Summary", f"- Duration: {duration_seconds:.1f}s"]
    if not json_enabled:
        lines.append("- Commands: (json disabled)")
        lines.append("- Files read: (json disabled)")
        lines.append("- Files changed: (json disabled)")
        return "\n".join(lines) + "\n"
    if not events:
        lines.append("- Commands: (no events captured)")
        lines.append("- Files read: (no events captured)")
        lines.append("- Files changed: (no events captured)")
        return "\n".join(lines) + "\n"

    commands: List[str] = []
    read_files: List[str] = []
    changed_files: List[str] = []

    for event in events:
        cmd = extract_command_detail(event)
        if cmd:
            commands.append(cmd)
            paths = extract_paths_from_command(cmd)
            read_files.extend(paths["read"])
            changed_files.extend(paths["write"])

        item = event.get("item")
        if isinstance(item, dict):
            item_type = str(item.get("type") or "").lower()
            if item_type in ("file_write", "file_edit", "file_change", "patch", "apply_patch"):
                changed_files.extend(extract_paths_from_event(event))

    commands = unique_preserve([safe_truncate(c, 240) for c in commands])[:max_items]
    read_files = unique_preserve(read_files)[:max_items]
    changed_files = unique_preserve(changed_files)[:max_items]

    if commands:
        lines.append(f"- Commands ({len(commands)}):")
        for cmd in commands:
            lines.append(f"  - {cmd}")
    else:
        lines.append("- Commands: (none captured)")

    if read_files:
        lines.append(f"- Files read ({len(read_files)}):")
        for path in read_files:
            lines.append(f"  - {path}")
    else:
        lines.append("- Files read: (none captured)")

    if changed_files:
        lines.append(f"- Files changed ({len(changed_files)}):")
        for path in changed_files:
            lines.append(f"  - {path}")
    else:
        lines.append("- Files changed: (none captured)")

    return "\n".join(lines) + "\n"


def format_json_event(event: Dict[str, Any], color_enabled: bool) -> Optional[str]:
    event_type = (
        event.get("type")
        or event.get("event")
        or event.get("kind")
        or event.get("event_type")
        or event.get("name")
        or "event"
    )
    label = categorize_event(str(event_type), event)
    if label == "THINKING":
        summary = extract_reasoning_summary(event)
        if summary:
            return colorize(label, summary, color_enabled)
        return colorize(label, "progress update (details hidden)", color_enabled)
    if label == "AGENT":
        item = event.get("item")
        if isinstance(item, dict):
            text = item.get("text")
            if isinstance(text, str) and text.strip():
                return colorize(label, text.strip(), color_enabled)
            content_text = extract_text_from_content(item.get("content"))
            if content_text:
                return colorize(label, content_text, color_enabled)
    detail = extract_command_detail(event)
    if label == "COMMAND" or detail:
        if detail:
            status = extract_command_status(event)
            suffix = f" [{status}]" if status else ""
            return colorize("COMMAND", detail + suffix, color_enabled)
    summary = summarize_event(event)
    message = summary if summary else str(event_type)
    return colorize(label, message, color_enabled)


def resolve_workspace_path(path: str) -> str:
    if not path or not path.strip():
        raise ValueError("Workspace path is empty.")
    return os.path.abspath(path)


def ensure_parent_dir(path: str) -> None:
    parent = os.path.dirname(path)
    if parent:
        os.makedirs(parent, exist_ok=True)


def ensure_memory_dir(path: str) -> None:
    if path:
        os.makedirs(path, exist_ok=True)


def ensure_plan_file(path: str) -> bool:
    if os.path.exists(path):
        return True
    template = """# TauriHands PLAN
# Write the next batch of tasks for Codex here.
# Keep scope small; prefer incremental changes with tests.
# The loop will synthesize a multi-role collaboration (product + UI + backend + QA)
# and apply code/doc updates each iteration.
#
# References:
# - AGENTS.md (constraints + module map)
# - CONCEPT.md (product blueprint)
# - README.md (overview + run commands)
# - PROTOCOLS.md (execution protocols)
# - UI_STRUCTURE.md (UI layout map)
# - src/pages/ConsolePage.vue (cockpit layout)
# - src-tauri/src/services/kernel.rs (loop + actions)
#
# Example tasks:
# - Fix a UI regression in src/components/PlanPanel.vue.
# - Improve tool output mapping in src/agents/orchestrator.ts.
# - Tighten task validation in src/stores/mission.ts.
# - Improve audit logging in src-tauri/src/services/audit.rs.
"""
    ensure_parent_dir(path)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(template)
    print(f"Created {path}. Edit it and rerun the script.")
    return False


def resolve_codex_path() -> Optional[str]:
    path = shutil.which("codex")
    if path:
        return path
    try:
        output = subprocess.check_output(
            ["where", "codex"],
            text=True,
            encoding="utf-8",
            errors="ignore",
        )
        for line in output.splitlines():
            line = line.strip()
            if line:
                return line
    except Exception:
        pass
    try:
        output = subprocess.check_output(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                "Get-Command codex | Select-Object -ExpandProperty Source",
            ],
            text=True,
            encoding="utf-8",
            errors="ignore",
        )
        for line in output.splitlines():
            line = line.strip()
            if line:
                return line
    except Exception:
        pass
    return None


def ensure_codex_available() -> str:
    path = resolve_codex_path()
    if not path:
        raise RuntimeError("codex command not found in PATH.")
    return path


def read_file_safe(path: str) -> str:
    if os.path.exists(path):
        with open(path, "r", encoding="utf-8") as handle:
            return handle.read()
    return ""


def read_memory_snapshot(path: str) -> Optional[Dict[str, Any]]:
    if not os.path.exists(path):
        return None
    try:
        with open(path, "r", encoding="utf-8") as handle:
            return json.load(handle)
    except Exception:
        return None


def write_memory_snapshot(path: str, data: Dict[str, Any]) -> None:
    if not path:
        return
    try:
        with open(path, "w", encoding="utf-8") as handle:
            json.dump(data, handle, ensure_ascii=False, indent=2)
    except Exception:
        pass


def read_lancedb_memory(db_dir: str, max_items: int = 6) -> List[Dict[str, Any]]:
    if not db_dir:
        return []
    try:
        import lancedb  # type: ignore
    except Exception:
        return []
    if not os.path.exists(db_dir):
        return []
    try:
        db = lancedb.connect(db_dir)
    except Exception:
        return []
    table_name = "codex_memory"
    try:
        table_names = set(db.table_names())
    except Exception:
        table_names = set()
    if table_name not in table_names:
        return []
    try:
        table = db.open_table(table_name)
        df = table.to_pandas()
    except Exception:
        return []
    if df is None or df.empty:
        return []
    sort_key = None
    for key in ("created_at", "last_run_at", "run_index"):
        if key in df.columns:
            sort_key = key
            break
    if sort_key:
        df = df.sort_values(by=sort_key, kind="mergesort")
    tail = df.tail(max_items)
    records: List[Dict[str, Any]] = []
    for _, row in tail.iterrows():
        record: Dict[str, Any] = {}
        for key in ("created_at", "last_run_at", "run_index", "target", "summary", "next_step", "error"):
            if key in row and isinstance(row[key], (str, int, float)) and str(row[key]).strip():
                record[key] = str(row[key]).strip()
        records.append(record)
    return records


def format_lancedb_memory(records: List[Dict[str, Any]]) -> List[str]:
    lines: List[str] = []
    for record in records:
        parts: List[str] = []
        if record.get("created_at"):
            parts.append(record["created_at"])
        elif record.get("last_run_at"):
            parts.append(record["last_run_at"])
        if record.get("target"):
            parts.append(f"target: {record['target']}")
        if record.get("summary"):
            parts.append(f"summary: {record['summary']}")
        if record.get("next_step"):
            parts.append(f"next: {record['next_step']}")
        if record.get("error"):
            parts.append(f"error: {record['error']}")
        if parts:
            lines.append("- " + " | ".join(parts))
    return lines


def get_memory_summary(
    snapshot: Optional[Dict[str, Any]],
    max_chars: int,
    memory_backend: str,
    memory_dir: str,
) -> str:
    lines: List[str] = []
    if snapshot:
        if snapshot.get("last_run_at"):
            lines.append(f"- last_run_at: {snapshot['last_run_at']}")
        if snapshot.get("last_summary"):
            lines.append(f"- last_summary: {snapshot['last_summary']}")
        if snapshot.get("last_next_step"):
            lines.append(f"- last_next_step: {snapshot['last_next_step']}")
        if snapshot.get("last_log_path"):
            lines.append(f"- last_log_path: {snapshot['last_log_path']}")

    if memory_backend in ("lancedb", "both"):
        lancedb_lines = format_lancedb_memory(read_lancedb_memory(memory_dir))
        if lancedb_lines:
            lines.append("Recent runs (LanceDB):")
            lines.extend(lancedb_lines)

    if not lines:
        return "none"
    text = "\n".join(lines)
    if max_chars > 0 and len(text) > max_chars:
        return text[:max_chars] + "..."
    return text


def extract_field(text: str, patterns: List[str]) -> str:
    if not text:
        return ""
    for pattern in patterns:
        try:
            match = re.search(pattern, text, flags=re.MULTILINE)
        except re.error:
            match = None
        if match:
            return match.group(1).strip()
    return ""


def write_loop_warning(log_path: str, message: str) -> None:
    if log_path:
        with open(log_path, "a", encoding="utf-8") as handle:
            handle.write(f"[WARN] {message}\n")
    print(f"WARNING: {message}")


def get_port_status(port: int) -> str:
    pid = None
    pname = ""
    try:
        import psutil  # type: ignore

        for conn in psutil.net_connections(kind="tcp"):
            if conn.laddr and conn.laddr.port == port:
                pid = conn.pid
                break
        if pid:
            try:
                pname = psutil.Process(pid).name()
            except Exception:
                pname = ""
    except Exception:
        pid = None

    if pid:
        if pname:
            return f"in use by {pname} (PID {pid})"
        return f"in use (PID {pid})"

    try:
        output = subprocess.check_output(
            ["netstat", "-ano", "-p", "tcp"],
            text=True,
            encoding="utf-8",
            errors="ignore",
        )
        for line in output.splitlines():
            line = line.strip()
            if not line.startswith("TCP"):
                continue
            parts = re.split(r"\s+", line)
            if len(parts) < 5:
                continue
            local_addr = parts[1]
            if local_addr.endswith(f":{port}"):
                pid = parts[4]
                return f"in use (PID {pid})"
    except Exception:
        pass

    return "free"


def get_port_summary() -> str:
    lines = [f"- {port}: {get_port_status(port)}" for port in PORTS]
    return "\n".join(lines)


def is_port_free(port: int) -> bool:
    return get_port_status(port).startswith("free")


def find_free_port(start_port: int, max_offset: int = 50) -> Optional[int]:
    for offset in range(1, max_offset + 1):
        candidate = start_port + offset
        if is_port_free(candidate):
            return candidate
    return None


def plan_port_policy(policy: str) -> Dict[str, Any]:
    normalized = (policy or "auto").strip().lower()
    in_use = {port for port in PORTS if not is_port_free(port)}
    overrides: Dict[str, str] = {}
    notes: List[str] = []
    skips: List[int] = []

    if not in_use or normalized in ("none", "off", "disabled", "false", "0"):
        return {"policy": normalized, "overrides": overrides, "notes": notes, "skips": skips}

    for port in PORTS:
        if port in in_use:
            skips.append(port)

    if skips:
        notes.append(f"- Dev ports in use: {', '.join(str(p) for p in skips)}")
        notes.append("- Tauri dev expects http://localhost:1420 (tauri.conf.json devUrl).")
        notes.append("- Vite uses strictPort=true with HMR on 1421; avoid starting a second dev server.")
        notes.append("- Stop the existing process or use -KillOnPortConflict if you intend to replace it.")

    return {"policy": normalized, "overrides": overrides, "notes": notes, "skips": skips}


def stop_port_process(port: int) -> bool:
    pid = None
    try:
        import psutil  # type: ignore

        for conn in psutil.net_connections(kind="tcp"):
            if conn.laddr and conn.laddr.port == port:
                pid = conn.pid
                break
        if pid and pid != os.getpid():
            # Avoid killing unrelated services (e.g., databases) by default.
            name = get_process_name(pid) or ""
            allow_names = {"node", "node.exe", "npm", "npm.cmd", "pnpm", "pnpm.exe", "pnpm.cmd", "yarn", "yarn.cmd", "bun", "bun.exe", "vite", "vite.cmd"}
            if name and name.lower() not in allow_names:
                return False
            try:
                psutil.Process(pid).kill()
                return True
            except Exception:
                return False
    except Exception:
        pid = None

    try:
        output = subprocess.check_output(
            ["netstat", "-ano", "-p", "tcp"],
            text=True,
            encoding="utf-8",
            errors="ignore",
        )
        for line in output.splitlines():
            line = line.strip()
            if not line.startswith("TCP"):
                continue
            parts = re.split(r"\s+", line)
            if len(parts) < 5:
                continue
            local_addr = parts[1]
            if local_addr.endswith(f":{port}"):
                pid = parts[4]
                break
    except Exception:
        pid = None

    if pid and pid != str(os.getpid()):
        try:
            name = get_process_name(int(pid)) or ""
            allow_names = {"node", "node.exe", "npm", "npm.cmd", "pnpm", "pnpm.exe", "pnpm.cmd", "yarn", "yarn.cmd", "bun", "bun.exe", "vite", "vite.cmd"}
            if name and name.lower() not in allow_names:
                return False
            subprocess.check_call(
                ["taskkill", "/PID", str(pid), "/F"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            return True
        except Exception:
            return False
    return False


def get_numbered_options(text: str) -> List[Dict[str, Any]]:
    options = []
    if not text:
        return options
    pattern = re.compile(r"^\s*(\d{1,2})[\.\)\]:\-\u3001]\s+(.+)$")
    for line in text.splitlines():
        match = pattern.match(line)
        if match:
            options.append({"number": int(match.group(1)), "text": match.group(2).strip()})
    return options


def collect_section_lines(plan_text: str, header_prefix: str) -> List[str]:
    if not plan_text:
        return []
    lines = plan_text.splitlines()
    start = None
    for idx, line in enumerate(lines):
        if line.strip().startswith(header_prefix):
            start = idx + 1
            break
    if start is None:
        return []
    section_lines = []
    for line in lines[start:]:
        if line.strip().startswith("## "):
            break
        section_lines.append(line.rstrip())
    return section_lines


def parse_backlog_items(section_lines: List[str]) -> List[str]:
    items = []
    current: List[str] = []
    for line in section_lines:
        match = re.match(r"^\s*(\d+)\)\s+(.+)$", line)
        if match:
            if current:
                items.append("\n".join(current).strip())
            current = [match.group(0).strip()]
            continue
        if current:
            stripped = line.strip()
            if stripped.startswith("-") or line.startswith(" ") or line.startswith("\t"):
                current.append(stripped)
    if current:
        items.append("\n".join(current).strip())
    return items


def select_backlog_target(plan_text: str, last_index: int) -> Dict[str, Any]:
    docs_section = collect_section_lines(plan_text, "## Backlog A:")
    code_section = collect_section_lines(plan_text, "## Backlog B:")
    items = parse_backlog_items(docs_section) + parse_backlog_items(code_section)
    if not items:
        return {"index": last_index, "item": ""}
    next_index = (last_index + 1) % len(items)
    return {"index": next_index, "item": items[next_index]}


def has_decision_cue(text: str) -> bool:
    if not text:
        return False
    lower = text.lower()
    for word in ["choose", "select", "pick", "option", "reply with", "respond with", "which", "number"]:
        if word in lower:
            return True
    if re.search(r"[\?\uFF1F]", text):
        return True
    for pattern in [
        "\u9009\u62e9",
        "\u9009\u9879",
        "\u56de\u590d",
        "\u7f16\u53f7",
        "\u8bf7\u8f93\u5165",
        "\u8bf7\u9009",
        "\u8fdb\u5165\u54ea\u4e2a",
    ]:
        if re.search(pattern, text):
            return True
    if re.search(r"[\u4e00-\u9fff]", text):
        return True
    return False


def needs_decision(text: str) -> bool:
    options = get_numbered_options(text)
    if len(options) < 2:
        return False
    return has_decision_cue(text)


def build_decision_prompt(last_response: str, options: List[Dict[str, Any]]) -> str:
    option_lines = "\n".join([f"{opt['number']}. {opt['text']}" for opt in options])
    return f"""You are a decision-only helper.
Choose the single best option number to proceed.
Rules:
- Reply with only the number (e.g. 1).
- Do not include any other text.
- Do not modify files or run commands.

Assistant response:
<<<
{last_response}
>>>

Options:
{option_lines}
"""


def parse_decision_number(decision_text: str, options: List[Dict[str, Any]]) -> Optional[int]:
    if not decision_text:
        return None
    # Prefer a clean "N" line; fallback to a leading token.
    match = re.search(r"(?m)^\s*(\d{1,2})\s*$", decision_text)
    if not match:
        match = re.search(r"^\s*(\d{1,2})\b", decision_text.strip())
    if not match:
        return None
    num = int(match.group(0))
    if any(opt["number"] == num for opt in options):
        return num
    return None


def build_project_prompt(
    plan_text: str,
    memory_summary: str,
    port_summary: str,
    port_policy_note: str,
    target_note: str,
) -> str:
    return f"""You are a collaborative team:
- Product Owner / Producer (vision, scope, priorities)
- UX/UI Designer (flows, cockpit layout, visual polish)
- Frontend Engineer (Vue 3 + Vite + Tailwind)
- Backend Engineer (Rust + Tauri services)
- QA / Tester (test strategy, verification)

Workflow for each iteration:
1) Brief role notes (1-3 bullets each).
2) Agree on the smallest shippable change; apply code/doc updates.
3) Decision: confirm scope and priorities for this round.
4) Add/adjust QA notes: tests or checks for any changes.
5) End with a short Next Step.

Format:
- End with two lines:
  Summary: ...
  Next Step: ...

Constraints:
- Event stream is the single source of truth for UI state.
- Plan/Task changes must go through Kernel Action/Reducer (no direct UI mutation).
- If user confirmation is needed, enter AWAITING_USER and wait.
- Keep scope small and incremental.
- Use concrete headings and consistent terminology.
- Avoid exposing chain-of-thought; be concise.
- Use the memory summary to avoid repeating completed work.

Port guard:
- Tauri dev uses a fixed Vite port (1420) with strictPort=true and HMR on 1421.
- If 1420 or 1421 is in use, do NOT start another dev server.
- Prefer short, non-blocking commands; avoid long-running dev servers in loops.
- Use -KillOnPortConflict only when you intend to replace an existing dev server.

Encoding guardrail:
- When reading text files, always use UTF-8:
  `Get-Content -Encoding utf8` (or `-Raw -Encoding utf8`) and set
  `[Console]::OutputEncoding = [Text.Encoding]::UTF8` if needed.
  Optional session default (PowerShell):
  `$PSDefaultParameterValues['Get-Content:Encoding']='utf8'; $PSDefaultParameterValues['Set-Content:Encoding']='utf8'`

Context references (read before changes):
- AGENTS.md (constraints + module map)
- CONCEPT.md (product blueprint)
- README.md (overview, setup, run commands)
- PROTOCOLS.md (execution protocols)
- UI_STRUCTURE.md (UI layout map)

Port status snapshot:
{port_summary}
{port_policy_note}

Memory summary (previous run):
{memory_summary}

Plan:
{plan_text}

Round target (auto-selected):
{target_note}
"""


def build_continuation_prompt(
    plan_text: str,
    last_response: str,
    decision_number: int,
    memory_summary: str,
    port_summary: str,
    port_policy_note: str,
    target_note: str,
) -> str:
    return f"""You are a collaborative team:
- Product Owner / Producer (vision, scope, priorities)
- UX/UI Designer (flows, cockpit layout, visual polish)
- Frontend Engineer (Vue 3 + Vite + Tailwind)
- Backend Engineer (Rust + Tauri services)
- QA / Tester (test strategy, verification)

Workflow for each iteration:
1) Brief role notes (1-3 bullets each).
2) Agree on the smallest shippable change; apply code/doc updates.
3) Decision: confirm scope and priorities for this round.
4) Add/adjust QA notes: tests or checks for any changes.
5) End with a short Next Step.

Format:
- End with two lines:
  Summary: ...
  Next Step: ...

Constraints:
- Event stream is the single source of truth for UI state.
- Plan/Task changes must go through Kernel Action/Reducer (no direct UI mutation).
- If user confirmation is needed, enter AWAITING_USER and wait.
- Keep scope small and incremental.
- Use concrete headings and consistent terminology.
- Avoid exposing chain-of-thought; be concise.
- Use the memory summary to avoid repeating completed work.

Port guard:
- Tauri dev uses a fixed Vite port (1420) with strictPort=true and HMR on 1421.
- If 1420 or 1421 is in use, do NOT start another dev server.
- Prefer short, non-blocking commands; avoid long-running dev servers in loops.
- Use -KillOnPortConflict only when you intend to replace an existing dev server.

Encoding guardrail:
- When reading text files, always use UTF-8:
  `Get-Content -Encoding utf8` (or `-Raw -Encoding utf8`) and set
  `[Console]::OutputEncoding = [Text.Encoding]::UTF8` if needed.
  Optional session default (PowerShell):
  `$PSDefaultParameterValues['Get-Content:Encoding']='utf8'; $PSDefaultParameterValues['Set-Content:Encoding']='utf8'`

Context references (read before changes):
- AGENTS.md (constraints + module map)
- CONCEPT.md (product blueprint)
- README.md (overview, setup, run commands)
- PROTOCOLS.md (execution protocols)
- UI_STRUCTURE.md (UI layout map)

Port status snapshot:
{port_summary}
{port_policy_note}

Memory summary (previous run):
{memory_summary}

Plan:
{plan_text}

Round target (auto-selected):
{target_note}

Previous assistant response:
<<<
{last_response}
>>>

User decision: {decision_number}

Continue with the selected option. If more choices are required, ask again using a numbered list.
"""


def build_repair_prompt(plan_text: str, last_response: str, reason: str) -> str:
    return f"""You are a repair-only helper.
The last run hit an error and must be fixed automatically.

Reason:
{reason}

Plan:
{plan_text}

Last assistant response:
<<<
{last_response}
>>>

Instructions:
- Diagnose the error.
- Apply minimal fixes in the repo.
- Run any necessary checks if they are cheap.
- Do not ask questions unless blocked.
- Summarize changes briefly at the end.
"""


def build_codex_command(base_args: List[str], codex_path: str) -> List[str]:
    ext = os.path.splitext(codex_path)[1].lower()
    if ext == ".ps1":
        return ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", codex_path] + base_args
    if ext in (".cmd", ".bat"):
        return ["cmd.exe", "/c", codex_path] + base_args
    return [codex_path] + base_args


def invoke_codex(
    prompt: str,
    output_file: str,
    workspace: str,
    show_output: bool,
    full_auto: bool,
    dangerous: bool,
    profile: str,
    heartbeat_seconds: int,
    codex_path: str,
    json_color: bool,
    json_log_path: str,
    log_path: str,
    resume_last: bool,
    event_collector: Optional[List[Dict[str, Any]]],
    extra_env: Optional[Dict[str, str]],
) -> int:
    agent_messages: List[str] = []
    rate_limit_sleep = 0
    if resume_last:
        # Resume does not accept --json/--full-auto/--dangerous flags.
        args = ["exec", "resume", "--last", "-"]
        cmd = build_codex_command(args, codex_path)
    else:
        args = ["exec", "--cd", workspace, "--output-last-message", output_file, "--color", "never"]
        if dangerous:
            args.append("--dangerously-bypass-approvals-and-sandbox")
        elif full_auto:
            args.append("--full-auto")
        if profile:
            args += ["--profile", profile]
        if json_color:
            args.append("--json")
        args.append("-")
        cmd = build_codex_command(args, codex_path)

    env = os.environ.copy()
    env.setdefault("PYTHONIOENCODING", "utf-8")
    if extra_env:
        env.update(extra_env)

    if not show_output and not json_color and not resume_last:
        result = subprocess.run(
            cmd,
            input=prompt,
            text=True,
            encoding="utf-8",
            errors="replace",
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=env,
            cwd=workspace,
        )
        return result.returncode

    proc = subprocess.Popen(
        cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
        cwd=workspace,
    )
    color_enabled = supports_color()

    def stream_json(stream):
        streaming_output = False
        for line in iter(stream.readline, ""):
            if json_log_path:
                try:
                    with open(json_log_path, "a", encoding="utf-8") as handle:
                        handle.write(line)
                except Exception:
                    pass
            stripped = line.strip()
            if not stripped:
                continue
            try:
                event = json.loads(stripped)
            except Exception:
                sys.stdout.write(line)
                sys.stdout.flush()
                append_log_raw(log_path, line)
                continue
            if event_collector is not None and len(event_collector) < 2000:
                event_collector.append(event)
            if resume_last:
                item = event.get("item")
                if isinstance(item, dict):
                    itype = str(item.get("type") or "")
                    if itype in ("agent_message", "assistant_message"):
                        text = item.get("text") or extract_text_from_content(item.get("content"))
                        if isinstance(text, str) and text.strip():
                            agent_messages.append(text.strip())
            delta_text = extract_stream_text(event)
            if delta_text:
                if show_output:
                    sys.stdout.write(delta_text)
                    sys.stdout.flush()
                    streaming_output = True
                append_log_raw(log_path, delta_text)
                if is_output_done_event(event):
                    if show_output and not delta_text.endswith("\n"):
                        sys.stdout.write("\n")
                        sys.stdout.flush()
                    if not delta_text.endswith("\n"):
                        append_log_raw(log_path, "\n")
                    streaming_output = False
                continue
            formatted = format_json_event(event, color_enabled)
            if formatted:
                if streaming_output:
                    sys.stdout.write("\n")
                    sys.stdout.flush()
                    append_log_raw(log_path, "\n")
                    streaming_output = False
                sys.stdout.write(formatted + "\n")
                sys.stdout.flush()
                append_log_raw(log_path, strip_ansi(formatted) + "\n")

    def stream_stdout(stream):
        for line in iter(stream.readline, ""):
            if resume_last:
                agent_messages.append(line)
            if show_output:
                sys.stdout.write(line)
                sys.stdout.flush()
            append_log_raw(log_path, line)

    def stream_stderr(stream):
        nonlocal rate_limit_sleep
        for line in iter(stream.readline, ""):
            if not line:
                continue
            text = strip_ansi(line.rstrip())
            if not text:
                continue
            label = "INFO" if is_ignorable_error_line(text) else "ERROR"
            if "usage_limit_reached" in text or "Too Many Requests" in text or "http 429" in text:
                seconds = extract_rate_limit_seconds(text)
                if seconds > rate_limit_sleep:
                    rate_limit_sleep = seconds
            sys.stdout.write(colorize(label, text, color_enabled) + "\n")
            sys.stdout.flush()
            append_log_raw(log_path, f"[{label}] {text}\n")

    threads = []
    if proc.stdout:
        if json_color:
            t_out = threading.Thread(target=stream_json, args=(proc.stdout,), daemon=True)
        else:
            t_out = threading.Thread(target=stream_stdout, args=(proc.stdout,), daemon=True)
        t_out.start()
        threads.append(t_out)
    if proc.stderr:
        t_err = threading.Thread(target=stream_stderr, args=(proc.stderr,), daemon=True)
        t_err.start()
        threads.append(t_err)

    if proc.stdin:
        try:
            proc.stdin.write(prompt)
            proc.stdin.close()
        except (BrokenPipeError, OSError):
            try:
                proc.stdin.close()
            except Exception:
                pass

    last_beat = time.time()
    while proc.poll() is None:
        if heartbeat_seconds > 0:
            now = time.time()
            if now - last_beat >= heartbeat_seconds:
                print(f"...running ({int(now - last_beat)}s heartbeat)")
                last_beat = now
        time.sleep(0.5)

    for t in threads:
        t.join(timeout=1)

    if resume_last:
        ensure_parent_dir(output_file)
        try:
            with open(output_file, "w", encoding="utf-8") as handle:
                handle.write("".join(agent_messages))
        except Exception:
            pass

    return_code = proc.returncode
    if rate_limit_sleep > 0:
        wait_seconds = rate_limit_sleep + 5
        sys.stdout.write(
            colorize("INFO", f"Rate limit hit. Sleeping {wait_seconds}s before retry.", color_enabled) + "\n"
        )
        sys.stdout.flush()
        time.sleep(wait_seconds)
        return_code = 0

    return return_code


def has_error_cue(text: str) -> bool:
    if not text:
        return False
    filtered_lines: List[str] = []
    for line in text.splitlines():
        lowered = line.lower()
        if (
            "usage_limit_reached" in lowered
            or "too many requests" in lowered
            or "http 429" in lowered
            or "usage limit" in lowered
        ):
            continue
        if is_ignorable_error_line(line):
            continue
        filtered_lines.append(line)
    text = "\n".join(filtered_lines).strip()
    if not text:
        return False
    patterns = [
        r"\btraceback\b",
        r"\bexception\b",
        r"\bpanic\b",
        r"\bfatal\b",
        r"\bsegmentation\s+fault\b",
        r"\btests?\s+failed\b",
        r"\bbuild\s+failed\b",
        r"\bcompilation\s+failed\b",
        r"\blint\s+failed\b",
        r"\bmodule\s+not\s+found\b",
        r"\bnon[-\s]?zero\b",
        r"\bexit\s+code\b",
        r"\u5931\u8d25",
        r"\u9519\u8bef",
        r"\u5f02\u5e38",
        r"\u65e0\u6cd5",
        r"\u627e\u4e0d\u5230",
        r"\u9519\u8bef\u7801",
    ]
    for pattern in patterns:
        try:
            if re.search(pattern, text, flags=re.IGNORECASE):
                return True
        except re.error:
            continue
    return False


def invoke_decision_loop(
    last_response: str,
    plan_path: str,
    log_path: str,
    last_message_path: str,
    decision_message_path: str,
    workspace: str,
    decision_rounds: int,
    auto_decide: bool,
    show_output: bool,
    full_auto: bool,
    dangerous: bool,
    profile: str,
    heartbeat_seconds: int,
    memory_summary: str,
    port_summary: str,
    port_policy_note: str,
    target_note: str,
    codex_path: str,
    json_color: bool,
    json_log_path: str,
    resume_last: bool,
    event_collector: Optional[List[Dict[str, Any]]],
    extra_env: Optional[Dict[str, str]],
) -> Dict[str, Any]:
    current = last_response
    exit_code = 0

    if not auto_decide:
        return {"last": current, "exit_code": 0}

    for round_index in range(1, decision_rounds + 1):
        if not needs_decision(current):
            break

        options = get_numbered_options(current)
        if len(options) < 2:
            break

        with open(log_path, "a", encoding="utf-8") as handle:
            handle.write(f"\n### Auto-decision round {round_index}\n")

        decision_prompt = build_decision_prompt(current, options)
        decision_exit = invoke_codex(
            decision_prompt,
            decision_message_path,
            workspace,
            show_output,
            full_auto,
            dangerous,
            profile,
            heartbeat_seconds,
            codex_path,
            json_color,
            json_log_path,
            log_path,
            resume_last,
            event_collector,
            extra_env,
        )
        decision_text = read_file_safe(decision_message_path).strip()
        with open(log_path, "a", encoding="utf-8") as handle:
            if decision_text:
                handle.write(strip_ansi(decision_text) + "\n")

        if decision_exit != 0:
            exit_code = decision_exit
            break

        decision_number = parse_decision_number(decision_text, options)
        if not decision_number:
            decision_number = options[0]["number"]
            with open(log_path, "a", encoding="utf-8") as handle:
                handle.write(f"Decision parse failed. Defaulting to {decision_number}.\n")

        plan_text = read_file_safe(plan_path)
        continue_prompt = build_continuation_prompt(
            plan_text,
            current,
            decision_number,
            memory_summary,
            port_summary,
            port_policy_note,
            target_note,
        )
        exit_code = invoke_codex(
            continue_prompt,
            last_message_path,
            workspace,
            show_output,
            full_auto,
            dangerous,
            profile,
            heartbeat_seconds,
            codex_path,
            json_color,
            json_log_path,
            log_path,
            resume_last,
            event_collector,
            extra_env,
        )
        current = read_file_safe(last_message_path)
        with open(log_path, "a", encoding="utf-8") as handle:
            if current.strip():
                handle.write(strip_ansi(current.strip()) + "\n")

        if exit_code != 0:
            break

    return {"last": current, "exit_code": exit_code}


def invoke_lancedb_store(db_dir: str, json_path: str, log_path: str) -> None:
    if not db_dir or not json_path:
        return
    script_path = os.path.join(os.getcwd(), "scripts", "memory", "lancedb_store.py")
    if not os.path.exists(script_path):
        write_loop_warning(log_path, f"LanceDB store script missing: {script_path}")
        return
    try:
        subprocess.check_call(
            [sys.executable, script_path, "--db", db_dir, "--json", json_path],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception as exc:
        write_loop_warning(log_path, f"LanceDB store failed: {exc}")


def invoke_iteration(state: Dict[str, Any], index: int, is_last: bool) -> bool:
    log_path = state["log_full"]
    run_start = time.time()
    stamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    with open(log_path, "a", encoding="utf-8") as handle:
        handle.write(f"\n## Run {index} - {stamp}\n")

    if state["kill_on_port_conflict"]:
        for port in PORTS:
            if stop_port_process(port):
                with open(log_path, "a", encoding="utf-8") as handle:
                    handle.write(f"[INFO] Killed process on port {port}.\n")

    port_summary = get_port_summary()
    port_plan = plan_port_policy(state["port_policy"])
    port_policy_note = ""
    if port_plan.get("policy"):
        header = f"Port policy: {port_plan['policy']}"
        if port_plan.get("notes"):
            port_policy_note = header + "\n" + "\n".join(port_plan["notes"])
        else:
            port_policy_note = header
    if port_policy_note:
        with open(log_path, "a", encoding="utf-8") as handle:
            handle.write(port_policy_note + "\n")

    plan_text = read_file_safe(state["plan_full"]).strip()
    if not plan_text:
        with open(log_path, "a", encoding="utf-8") as handle:
            handle.write("Plan is empty. Aborting.\n")
        return False

    target_note = "none"
    target_index = -1
    if state.get("auto_pick_target"):
        last_target_index = -1
        snapshot = state.get("memory_snapshot") or {}
        if isinstance(snapshot, dict):
            last_target_index = int(snapshot.get("last_target_index", -1))
        target_pick = select_backlog_target(plan_text, last_target_index)
        target_note = target_pick.get("item") or "none"
        target_index = target_pick.get("index", last_target_index)
        if target_note != "none":
            with open(log_path, "a", encoding="utf-8") as handle:
                handle.write(f"[INFO] Auto-target: {target_note}\n")

    memory_summary = (
        get_memory_summary(
            state.get("memory_snapshot"),
            state["memory_max_chars"],
            state["memory_backend"],
            state["memory_dir_full"],
        )
        if state["memory_enabled"]
        else "disabled"
    )
    prompt = build_project_prompt(plan_text, memory_summary, port_summary, port_policy_note, target_note)
    event_collector: List[Dict[str, Any]] = []
    extra_env = port_plan.get("overrides") if isinstance(port_plan, dict) else None

    exit_code = invoke_codex(
        prompt,
        state["last_full"],
        state["workspace_full"],
        state["show_output"],
        state["full_auto"],
        state["dangerous"],
        state["profile"],
        state["heartbeat_seconds"],
        state["codex_path"],
        state["json_color"],
        state["json_log_full"],
        log_path,
        state["resume_last"],
        event_collector,
        extra_env,
    )

    last = read_file_safe(state["last_full"]).strip()
    if last:
        with open(log_path, "a", encoding="utf-8") as handle:
            handle.write(strip_ansi(last) + "\n")

    decision_result = invoke_decision_loop(
        last,
        state["plan_full"],
        log_path,
        state["last_full"],
        state["decision_full"],
        state["workspace_full"],
        state["decision_rounds"],
        state["auto_decide"],
        state["show_output"],
        state["full_auto"],
        state["dangerous"],
        state["profile"],
        state["heartbeat_seconds"],
        memory_summary,
        port_summary,
        port_policy_note,
        target_note,
        state["codex_path"],
        state["json_color"],
        state["json_log_full"],
        state["resume_last"],
        event_collector,
        extra_env,
    )
    last = decision_result["last"]
    if decision_result["exit_code"] != 0:
        exit_code = decision_result["exit_code"]

    needs_repair = state["auto_repair"] and (exit_code != 0 or has_error_cue(last))
    if needs_repair:
        for r_index in range(1, state["repair_rounds"] + 1):
            with open(log_path, "a", encoding="utf-8") as handle:
                handle.write(f"\n### Auto-repair round {r_index}\n")
            reason = f"Codex exited with code {exit_code}" if exit_code != 0 else "Error indicators found in assistant output."
            repair_prompt = build_repair_prompt(prompt, last, reason)
            repair_exit = invoke_codex(
                repair_prompt,
                state["repair_full"],
                state["workspace_full"],
                state["show_output"],
                state["full_auto"],
                state["dangerous"],
                state["profile"],
                state["heartbeat_seconds"],
                state["codex_path"],
                state["json_color"],
                state["json_log_full"],
                log_path,
                state["resume_last"],
                event_collector,
                extra_env,
            )
            repair_text = read_file_safe(state["repair_full"]).strip()
            if repair_text:
                with open(log_path, "a", encoding="utf-8") as handle:
                    handle.write(strip_ansi(repair_text) + "\n")

            if repair_exit != 0:
                with open(log_path, "a", encoding="utf-8") as handle:
                    handle.write(f"Repair Codex exited with code {repair_exit}.\n")
                exit_code = repair_exit
                break

            if state["repair_delay_seconds"] > 0:
                time.sleep(state["repair_delay_seconds"])

            exit_code = invoke_codex(
                prompt,
                state["last_full"],
                state["workspace_full"],
                state["show_output"],
                state["full_auto"],
                state["dangerous"],
                state["profile"],
                state["heartbeat_seconds"],
                state["codex_path"],
                state["json_color"],
                state["json_log_full"],
                log_path,
                state["resume_last"],
                event_collector,
                extra_env,
            )
            last = read_file_safe(state["last_full"]).strip()
            if last:
                with open(log_path, "a", encoding="utf-8") as handle:
                    handle.write(strip_ansi(last) + "\n")

            decision_result = invoke_decision_loop(
                last,
                state["plan_full"],
                log_path,
                state["last_full"],
                state["decision_full"],
                state["workspace_full"],
                state["decision_rounds"],
                state["auto_decide"],
                state["show_output"],
                state["full_auto"],
                state["dangerous"],
                state["profile"],
                state["heartbeat_seconds"],
                memory_summary,
                port_summary,
                port_policy_note,
                target_note,
                state["codex_path"],
                state["json_color"],
                state["json_log_full"],
                state["resume_last"],
                event_collector,
                extra_env,
            )
            last = decision_result["last"]
            if decision_result["exit_code"] != 0:
                exit_code = decision_result["exit_code"]

            if exit_code == 0 and not has_error_cue(last):
                break

    if state["memory_enabled"]:
        summary_patterns = [
            r"(?im)^\s*(?:summary|change summary|brief summary|\u6982\u8981|\u603b\u7ed3)\s*[:\uFF1A]\s*(.+)$",
            r"(?im)^\s*\*{1,2}(?:summary|\u603b\u7ed3)\*{1,2}\s*[:\uFF1A]\s*(.+)$",
        ]
        next_patterns = [
            r"(?im)^\s*(?:next step|next steps|next\s*action|\u4e0b\u4e00\u6b65|\u540e\u7eed\u6b65\u9aa4)\s*[:\uFF1A]\s*(.+)$",
            r"(?im)^\s*\*{1,2}(?:next step|\u4e0b\u4e00\u6b65)\*{1,2}\s*[:\uFF1A]\s*(.+)$",
        ]
        summary = extract_field(last, summary_patterns)
        if not summary:
            flat = re.sub(r"\s+", " ", last.replace("\r", " ").replace("\n", " ")).strip()
            if len(flat) > 240:
                flat = flat[:240] + "..."
            summary = flat or "no output"
        next_step = extract_field(last, next_patterns)

        memory_data = {
            "last_run_at": datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
            "last_round_index": index,
            "last_target_index": target_index,
            "last_target": target_note,
            "last_summary": summary,
            "last_next_step": next_step,
            "last_log_path": log_path,
            "last_response_path": state["last_full"],
            "last_exit_code": exit_code,
        }
        if exit_code != 0:
            memory_data["last_error"] = f"exit code {exit_code}"
        elif has_error_cue(last):
            memory_data["last_error"] = "error indicators in output"

        write_memory_snapshot(state["memory_snapshot_path"], memory_data)
        state["memory_snapshot"] = memory_data
        if state["memory_backend"] in ("lancedb", "both"):
            invoke_lancedb_store(state["memory_dir_full"], state["memory_snapshot_path"], log_path)

    summary_block = summarize_run_events(
        event_collector,
        time.time() - run_start,
        bool(state["json_color"]),
    )
    with open(log_path, "a", encoding="utf-8") as handle:
        handle.write(summary_block)

    if exit_code != 0:
        with open(log_path, "a", encoding="utf-8") as handle:
            handle.write(f"Codex exited with code {exit_code}.\n")
        if state["continue_on_error"]:
            return True
        return False

    if state["delay_seconds"] > 0 and not is_last:
        time.sleep(state["delay_seconds"])

    return True


def main() -> int:
    parser = argparse.ArgumentParser(description="Codex loop for TauriHands (Python)")
    parser.add_argument("--plan-path", "-PlanPath", default="PLAN.md")
    parser.add_argument("--log-path", "-LogPath", default="RUNLOG.md")
    parser.add_argument("--last-message-path", "-LastMessagePath", default="LAST_RESPONSE.md")
    parser.add_argument("--decision-message-path", "-DecisionMessagePath", default="DECISION_RESPONSE.md")
    parser.add_argument("--workspace", "-Workspace", default=os.getcwd())
    parser.add_argument("--iterations", "-Iterations", type=int, default=1)
    parser.add_argument("--delay-seconds", "-DelaySeconds", type=int, default=0)
    parser.add_argument("--decision-rounds", "-DecisionRounds", type=int, default=3)
    parser.add_argument("--repair-message-path", "-RepairMessagePath", default="REPAIR_RESPONSE.md")
    parser.add_argument("--repair-rounds", "-RepairRounds", type=int, default=2)
    parser.add_argument("--repair-delay-seconds", "-RepairDelaySeconds", type=int, default=0)
    parser.add_argument("--auto-repair", "-AutoRepair", action="store_true", default=True)
    parser.add_argument("--no-auto-repair", "-NoAutoRepair", dest="auto_repair", action="store_false")
    parser.add_argument("--continue-on-error", "-ContinueOnError", action="store_true")
    parser.add_argument("--forever", "-Forever", action="store_true")
    parser.add_argument("--full-auto", "-FullAuto", action="store_true")
    parser.add_argument("--dangerous", "-Dangerous", action="store_true")
    parser.add_argument("--profile", "-Profile", default="")
    parser.add_argument("--auto-decide", "-AutoDecide", action="store_true", default=True)
    parser.add_argument("--no-auto-decide", "-NoAutoDecide", dest="auto_decide", action="store_false")
    parser.add_argument("--show-output", "-ShowOutput", action="store_true")
    parser.add_argument("--heartbeat-seconds", "-HeartbeatSeconds", type=int, default=20)
    parser.add_argument("--json-color", "-JsonColor", action="store_true")
    parser.add_argument("--json-log-path", "-JsonLogPath", default="")
    parser.add_argument("--kill-on-port-conflict", "-KillOnPortConflict", action="store_true")
    parser.add_argument("--port-policy", "-PortPolicy", default="auto")
    parser.add_argument("--ansi", "-Ansi", action="store_true")
    parser.add_argument("--no-ansi", "-NoAnsi", dest="ansi", action="store_false")
    parser.add_argument("--auto-pick-target", "-AutoPickTarget", action="store_true", default=True)
    parser.add_argument("--no-auto-pick-target", "-NoAutoPickTarget", dest="auto_pick_target", action="store_false")
    parser.add_argument("--resume-last", "-ResumeLast", action="store_true", default=False)
    parser.add_argument("--no-resume-last", "-NoResumeLast", dest="resume_last", action="store_false")
    parser.add_argument("--memory-backend", "-MemoryBackend", default="file")
    parser.add_argument("--memory-dir", "-MemoryDir", default=".taurihands/codex-loop/memory")
    parser.add_argument("--memory-max-chars", "-MemoryMaxChars", type=int, default=2000)

    args = parser.parse_args()

    codex_path = ensure_codex_available()
    workspace_full = resolve_workspace_path(args.workspace)
    os.chdir(workspace_full)

    plan_full = os.path.join(workspace_full, args.plan_path)
    log_full = os.path.join(workspace_full, args.log_path)
    last_full = os.path.join(workspace_full, args.last_message_path)
    decision_full = os.path.join(workspace_full, args.decision_message_path)
    repair_full = os.path.join(workspace_full, args.repair_message_path)
    json_log_full = ""
    if args.json_log_path:
        json_log_full = os.path.join(workspace_full, args.json_log_path)

    ensure_parent_dir(plan_full)
    ensure_parent_dir(log_full)
    ensure_parent_dir(last_full)
    ensure_parent_dir(decision_full)
    ensure_parent_dir(repair_full)
    if json_log_full:
        ensure_parent_dir(json_log_full)

    if not ensure_plan_file(plan_full):
        return 1

    memory_backend = (args.memory_backend or "file").lower()
    memory_enabled = memory_backend not in ("none", "off", "disabled", "false", "0")
    memory_dir_full = os.path.join(workspace_full, args.memory_dir)
    memory_snapshot_path = os.path.join(memory_dir_full, "last_state.json")
    memory_snapshot = None
    if memory_enabled:
        ensure_memory_dir(memory_dir_full)
        memory_snapshot = read_memory_snapshot(memory_snapshot_path)

    json_mode = bool(args.json_color) or bool(args.json_log_path)
    if args.resume_last and json_mode:
        json_mode = False
        print("NOTE: resume mode does not support --json; disabling JSON output.")
    if args.resume_last and (args.full_auto or args.dangerous or args.profile):
        print("NOTE: resume mode ignores --full-auto/--dangerous/--profile.")

    global ANSI_ENABLED
    ANSI_ENABLED = bool(args.ansi)

    state = {
        "workspace_full": workspace_full,
        "plan_full": plan_full,
        "log_full": log_full,
        "last_full": last_full,
        "decision_full": decision_full,
        "repair_full": repair_full,
        "delay_seconds": args.delay_seconds,
        "decision_rounds": args.decision_rounds,
        "repair_rounds": args.repair_rounds,
        "repair_delay_seconds": args.repair_delay_seconds,
        "auto_repair": bool(args.auto_repair),
        "continue_on_error": bool(args.continue_on_error),
        "full_auto": bool(args.full_auto) and not bool(args.dangerous),
        "dangerous": bool(args.dangerous),
        "profile": args.profile,
        "auto_decide": bool(args.auto_decide),
        "show_output": bool(args.show_output),
        "heartbeat_seconds": args.heartbeat_seconds,
        "json_color": json_mode,
        "json_log_full": json_log_full,
        "kill_on_port_conflict": bool(args.kill_on_port_conflict),
        "port_policy": args.port_policy,
        "ansi": bool(args.ansi),
        "auto_pick_target": bool(args.auto_pick_target),
        "resume_last": bool(args.resume_last),
        "memory_backend": memory_backend,
        "memory_enabled": memory_enabled,
        "memory_dir_full": memory_dir_full,
        "memory_snapshot_path": memory_snapshot_path,
        "memory_snapshot": memory_snapshot,
        "memory_max_chars": args.memory_max_chars,
        "codex_path": codex_path,
    }

    if args.forever:
        index = 1
        while True:
            if not invoke_iteration(state, index, False):
                break
            index += 1
    else:
        for index in range(1, args.iterations + 1):
            is_last = index >= args.iterations
            if not invoke_iteration(state, index, is_last):
                break

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
