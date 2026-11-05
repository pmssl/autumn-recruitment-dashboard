"""Track project skill use and RULE.md changes once per Codex turn."""

from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


SKILL_NAME = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def project_skills(repo: Path) -> list[str]:
    root = repo / ".agents" / "skills"
    if not root.is_dir():
        return []
    return sorted(path.name for path in root.iterdir() if (path / "SKILL.md").is_file())


def rule_hash(repo: Path) -> str:
    return hashlib.sha256((repo / "RULE.md").read_bytes()).hexdigest()


def committed_rule_changes(repo: Path) -> int:
    result = subprocess.run(
        ["git", "rev-list", "--count", "HEAD", "--", "RULE.md"],
        cwd=repo, capture_output=True, text=True, check=True,
    )
    return int(result.stdout.strip())


def latest_active_turn(sessions_dir: Path, thread_id: str) -> str:
    candidates = list(sessions_dir.rglob(f"*{thread_id}*.jsonl"))
    if not candidates:
        raise ValueError(f"找不到当前线程日志：{thread_id}")
    session = max(candidates, key=lambda path: path.stat().st_mtime)
    latest = None
    completed = set()
    with session.open(encoding="utf-8") as source:
        for line in source:
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("type") != "event_msg":
                continue
            payload = event.get("payload") or {}
            if payload.get("type") == "task_started":
                latest = payload.get("turn_id")
            elif payload.get("type") == "task_complete":
                completed.add(payload.get("turn_id"))
    if not latest or latest in completed:
        raise ValueError("没有找到正在进行的轮次；可传入 --turn-id 明确指定")
    return latest


def read_state(path: Path, current_hash: str | None = None) -> dict:
    if path.is_file():
        state = json.loads(path.read_text(encoding="utf-8"))
        if state.get("schema") != 1 or not isinstance(state.get("records"), list):
            raise ValueError("统计状态文件格式不受支持")
        return state
    return {"schema": 1, "baseline_rule_hash": current_hash, "records": []}


def update_state(state: dict, thread_id: str, turn_id: str, skills: list[str], current_hash: str) -> dict:
    records = state["records"]
    key = f"{thread_id}/{turn_id}"
    match = next((index for index, record in enumerate(records) if record["key"] == key), None)
    if match is not None and match != len(records) - 1:
        raise ValueError("只能重新记录最近一轮，避免改写后续轮次的 RULE.md 基线")
    if match is None:
        previous_hash = records[-1]["rule_hash"] if records else state["baseline_rule_hash"]
        records.append({
            "key": key,
            "recorded_at": datetime.now(timezone.utc).isoformat(),
            "skills": sorted(set(skills)),
            "rule_hash": current_hash,
            "rule_changed": current_hash != previous_hash,
        })
    else:
        previous_hash = records[match - 1]["rule_hash"] if match else state["baseline_rule_hash"]
        record = records[match]
        record["skills"] = sorted(set(record["skills"]) | set(skills))
        record["rule_hash"] = current_hash
        record["rule_changed"] = current_hash != previous_hash
        record["recorded_at"] = datetime.now(timezone.utc).isoformat()
    return records[match if match is not None else -1]


def write_state(path: Path, state: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", dir=path.parent, delete=False) as output:
        json.dump(state, output, ensure_ascii=False, indent=2)
        output.write("\n")
        temporary = Path(output.name)
    os.replace(temporary, path)


def summary(state: dict, repo: Path, current: dict | None = None) -> dict:
    counts: Counter[str] = Counter()
    for record in state["records"]:
        counts.update(record["skills"])
    for name in project_skills(repo):
        counts.setdefault(name, 0)
    return {
        "tracked_turns": len(state["records"]),
        "current_turn_skills": current["skills"] if current else None,
        "current_turn_rule_changed": current["rule_changed"] if current else None,
        "skill_uses_by_turn": dict(sorted(counts.items())),
        "rule_changed_turns_since_tracking": sum(bool(record["rule_changed"]) for record in state["records"]),
        "rule_commits_in_git_history": committed_rule_changes(repo),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="逐轮统计本项目 Skill 使用和 RULE.md 改动")
    parser.add_argument("action", choices=["record", "show"])
    parser.add_argument("--skill", action="append", default=[], help="本轮实际使用的项目 Skill，可重复指定")
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--state", type=Path, help="统计状态路径，默认在工具目录中")
    parser.add_argument("--sessions-dir", type=Path, default=Path.home() / ".codex" / "sessions")
    parser.add_argument("--thread-id", default=os.getenv("CODEX_THREAD_ID"))
    parser.add_argument("--turn-id", help="仅日志中无法自动识别时手动指定")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    repo = args.repo.resolve()
    state_path = args.state or Path(__file__).resolve().parent / "usage-state.json"
    try:
        current_hash = rule_hash(repo)
        state = read_state(state_path, current_hash)
        current = None
        if args.action == "record":
            available = set(project_skills(repo))
            for name in args.skill:
                if not SKILL_NAME.fullmatch(name) or name not in available:
                    raise ValueError(f"不是当前项目 Skill：{name}")
            if not args.thread_id:
                raise ValueError("缺少 CODEX_THREAD_ID；请指定 --thread-id 和 --turn-id")
            turn_id = args.turn_id or latest_active_turn(args.sessions_dir, args.thread_id)
            current = update_state(state, args.thread_id, turn_id, args.skill, current_hash)
            write_state(state_path, state)
        result = summary(state, repo, current)
    except (OSError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        parser.error(str(error))
    if args.json:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        if current:
            print(f"本轮：Skill {', '.join(current['skills']) or '无'}；RULE.md {'已改动' if current['rule_changed'] else '未改动'}")
        print(f"累计（启用后 {result['tracked_turns']} 轮）：" + "，".join(
            f"{name} {count} 次" for name, count in result["skill_uses_by_turn"].items()
        ))
        print(f"RULE.md：启用后改动 {result['rule_changed_turns_since_tracking']} 轮；Git 历史涉及该文件 {result['rule_commits_in_git_history']} 个提交")
    return 0


if __name__ == "__main__":
    sys.exit(main())
