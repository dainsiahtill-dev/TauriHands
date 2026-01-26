import argparse
import datetime
import hashlib
import json
import os
import sys
import uuid


def safe_str(value) -> str:
    if value is None:
        return ""
    text = str(value).strip()
    return text


def truncate(text: str, limit: int = 2000) -> str:
    if not text:
        return ""
    if limit <= 0 or len(text) <= limit:
        return text
    return text[:limit] + "..."


def build_record(snapshot: dict) -> dict:
    created_at = safe_str(snapshot.get("last_run_at"))
    if not created_at:
        created_at = datetime.datetime.utcnow().isoformat(timespec="seconds") + "Z"

    summary = safe_str(snapshot.get("last_summary"))
    next_step = safe_str(snapshot.get("last_next_step"))
    target = safe_str(snapshot.get("last_target"))
    error = safe_str(snapshot.get("last_error"))
    log_path = safe_str(snapshot.get("last_log_path"))
    response_path = safe_str(snapshot.get("last_response_path"))
    run_index = snapshot.get("last_round_index")

    parts = [summary, next_step, target, error]
    text = truncate(" | ".join([p for p in parts if p]), 4000)
    fingerprint_src = "|".join([created_at, summary, next_step, target, error])
    fingerprint = hashlib.sha256(fingerprint_src.encode("utf-8")).hexdigest()

    record = {
        "id": str(uuid.uuid4()),
        "created_at": created_at,
        "run_index": int(run_index) if isinstance(run_index, int) else None,
        "target": target,
        "summary": summary,
        "next_step": next_step,
        "error": error,
        "log_path": log_path,
        "response_path": response_path,
        "text": text,
        "fingerprint": fingerprint,
    }
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description="Store Codex loop memory in LanceDB")
    parser.add_argument("--db", required=True, help="LanceDB directory")
    parser.add_argument("--json", required=True, help="Path to last_state.json")
    parser.add_argument("--table", default="codex_memory", help="Table name")
    args = parser.parse_args()

    if not os.path.exists(args.json):
        print(f"Snapshot not found: {args.json}", file=sys.stderr)
        return 1

    try:
        with open(args.json, "r", encoding="utf-8") as handle:
            snapshot = json.load(handle)
    except Exception as exc:
        print(f"Failed to read snapshot: {exc}", file=sys.stderr)
        return 1

    try:
        import lancedb  # type: ignore
    except Exception as exc:
        print(f"LanceDB not available: {exc}", file=sys.stderr)
        return 1

    os.makedirs(args.db, exist_ok=True)
    record = build_record(snapshot)

    try:
        db = lancedb.connect(args.db)
        table_names = set(db.table_names())
        if args.table in table_names:
            table = db.open_table(args.table)
            try:
                df = table.to_pandas()
                if df is not None and not df.empty:
                    last = df.tail(1).iloc[0]
                    last_fp = str(last.get("fingerprint", ""))
                    if last_fp == record["fingerprint"]:
                        return 0
            except Exception:
                pass
            table.add([record])
        else:
            db.create_table(args.table, data=[record])
    except Exception as exc:
        print(f"LanceDB store failed: {exc}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
