#!/usr/bin/env python3
"""Config-driven quickstart TUI for local station/postal converter repos."""

from __future__ import annotations

import argparse
import curses
import json
import os
import shlex
import shutil
import signal
import socket
import subprocess
import sys
import textwrap
import threading
import time
import traceback
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


UI_STRINGS = {
    "en": {
        "ready": "Ready.",
        "launcher": "launcher",
        "language_bar": "Language: {japanese} / {english}  (l or Tab)",
        "header": "{title} {launcher}  |  DB: 1 postgres  2 mysql  3 sqlite  |  validate: v toggle  |  active {current_db} / {validate_mode} / env {env_db}",
        "db_mode_line": "DB mode: {db_mode} (env: {env_db}, validate: {validate_mode}, lang: {language})",
        "quick_path": "Quick Path",
        "quick_order": "Quick order",
        "kind": "Kind",
        "status": "Status",
        "url": "URL",
        "health": "Health",
        "current_step": "Current step",
        "step_elapsed": "Step elapsed",
        "last_run": "Last run",
        "started_at": "Started",
        "finished_at": "Finished",
        "last_exit_code": "Exit code",
        "recent_output": "Recent output:",
        "no_log_output": "(no log output yet)",
        "no_item_selected": "No item selected.",
        "footer": "Enter/s run  x stop/cancel  r restart  o open URL  v validate  l/Tab language  q quit",
        "selected": "selected",
        "unknown": "unknown",
        "running": "running",
        "starting": "starting",
        "not_started": "not started",
        "finished": "finished",
        "failed": "failed",
        "skipped": "skipped",
        "stopped": "stopped",
        "stopping": "stopping",
        "detached_ended": "detached session ended",
        "workflow_step": "workflow step: {step}",
        "port_open": "port {port} open",
        "port_closed": "port {port} closed",
        "pid": "pid {pid}",
        "container_not_created": "container not created",
        "no_container_configured": "no container configured",
        "logs_service_hint": "Logs are shown when the service is started from this TUI.",
        "docker_logs_failed": "docker logs failed: {error}",
        "already_running": "{label} is already running.",
        "started": "Started {label}.",
        "not_managed": "{label} is not managed by this TUI.",
        "stopping_item": "Stopping {label}...",
        "skipped_item": "Skipped {label}.",
        "stop_not_supported": "Stop is not supported for {label}.",
        "restart_services_only": "Restart is only available for services.",
        "no_url": "{label} has no URL.",
        "opened_url": "Opened {url}",
        "open_failed": "Open failed: {error}",
        "not_ready_yet": "{label} is not ready yet.",
        "db_mode_set": "DB mode set to {label}.",
        "validate_mode_set": "Validate mode set to {label}.",
        "language_set": "Language set to {language}.",
        "workflow_finished": "{label} finished.",
        "workflow_failed": "{label} stopped on a failed step.",
        "workflow_canceling": "Cancel requested for {label}.",
        "workflow_canceled": "{label} was canceled.",
        "failed_to_start_step": "failed to start step",
        "service_not_ready": "service did not become ready: {label}",
        "service_setup_step": "setup: {step}",
        "canceled": "canceled",
    },
    "ja": {
        "ready": "準備完了。",
        "launcher": "ランチャー",
        "language_bar": "Language: {japanese} / {english}  (l または Tab)",
        "header": "{title} {launcher}  |  DB: 1 postgres  2 mysql  3 sqlite  |  validate: v 切替  |  現在 {current_db} / {validate_mode} / env {env_db}",
        "db_mode_line": "DBモード: {db_mode} (env: {env_db}, validate: {validate_mode}, 言語: {language})",
        "quick_path": "クイックパス",
        "quick_order": "推奨順",
        "kind": "種別",
        "status": "状態",
        "url": "URL",
        "health": "Health",
        "current_step": "現在のステップ",
        "step_elapsed": "経過",
        "last_run": "直近の実行",
        "started_at": "開始",
        "finished_at": "終了",
        "last_exit_code": "終了コード",
        "recent_output": "直近の出力:",
        "no_log_output": "(まだログ出力はありません)",
        "no_item_selected": "項目が選択されていません。",
        "footer": "Enter/s 実行  x 停止/キャンセル  r 再起動  o URLを開く  v validate切替  l/Tab 言語切替  q 終了",
        "selected": "選択中",
        "unknown": "不明",
        "running": "実行中",
        "starting": "起動中",
        "not_started": "未実行",
        "finished": "完了",
        "failed": "失敗",
        "skipped": "スキップ",
        "stopped": "停止",
        "stopping": "停止中",
        "detached_ended": "前回のセッションは終了済み",
        "workflow_step": "ワークフロー進行中: {step}",
        "port_open": "port {port} は開いています",
        "port_closed": "port {port} は閉じています",
        "pid": "pid {pid}",
        "container_not_created": "コンテナはまだ作成されていません",
        "no_container_configured": "コンテナ設定がありません",
        "logs_service_hint": "このTUIから起動したときのログがここに表示されます。",
        "docker_logs_failed": "docker logs の取得に失敗しました: {error}",
        "already_running": "{label} はすでに起動しています。",
        "started": "{label} を起動しました。",
        "not_managed": "{label} はこのTUI管理下ではありません。",
        "stopping_item": "{label} を停止しています...",
        "skipped_item": "{label} をスキップしました。",
        "stop_not_supported": "{label} は停止操作に対応していません。",
        "restart_services_only": "再起動は service 項目だけに対応しています。",
        "no_url": "{label} にURLがありません。",
        "opened_url": "{url} を開きました",
        "open_failed": "開けませんでした: {error}",
        "not_ready_yet": "{label} はまだ ready ではありません。",
        "db_mode_set": "DBモードを {label} に切り替えました。",
        "validate_mode_set": "Validate mode を {label} に切り替えました。",
        "language_set": "表示言語を {language} に切り替えました。",
        "workflow_finished": "{label} が完了しました。",
        "workflow_failed": "{label} は途中の失敗で停止しました。",
        "workflow_canceling": "{label} のキャンセルを受け付けました。",
        "workflow_canceled": "{label} をキャンセルしました。",
        "failed_to_start_step": "ステップの起動に失敗しました",
        "service_not_ready": "service の ready 待ちに失敗しました: {label}",
        "service_setup_step": "セットアップ中: {step}",
        "canceled": "キャンセル",
    },
}


LANGUAGE_NAMES = {
    "en": {"en": "English", "ja": "英語"},
    "ja": {"en": "Japanese", "ja": "日本語"},
}


def now_iso() -> str:
    return datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds")


def elapsed_seconds_since(iso_timestamp: Optional[str]) -> Optional[int]:
    if not iso_timestamp:
        return None
    try:
        started = datetime.fromisoformat(iso_timestamp)
    except ValueError:
        return None
    return max(0, int((datetime.now(timezone.utc).astimezone() - started).total_seconds()))


def format_elapsed(seconds: Optional[int]) -> str:
    if seconds is None:
        return "?"
    minutes, remaining = divmod(seconds, 60)
    hours, minutes = divmod(minutes, 60)
    if hours:
        return f"{hours:d}:{minutes:02d}:{remaining:02d}"
    return f"{minutes:d}:{remaining:02d}"


def load_json(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def save_json(path: Path, payload: Dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = path.with_suffix(".tmp")
    with tmp_path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, ensure_ascii=True, indent=2, sort_keys=True)
    tmp_path.replace(path)


def wrap_text(text: str, width: int) -> List[str]:
    if width <= 1:
        return [text[: max(width, 1)]]
    lines: List[str] = []
    for paragraph in text.splitlines() or [""]:
        if not paragraph:
            lines.append("")
            continue
        lines.extend(
            textwrap.wrap(
                paragraph,
                width=width,
                replace_whitespace=False,
                drop_whitespace=False,
            )
            or [""]
        )
    return lines


def read_tail(path: Path, max_lines: int = 80, max_chars: int = 16000) -> str:
    if not path.is_file():
        return ""

    with path.open("rb") as handle:
        handle.seek(0, os.SEEK_END)
        size = handle.tell()
        chunk_size = 4096
        data = bytearray()
        while size > 0 and data.count(b"\n") <= max_lines:
            read_size = min(chunk_size, size)
            size -= read_size
            handle.seek(size)
            data[:0] = handle.read(read_size)
            if len(data) > max_chars * 2:
                data = data[-max_chars * 2 :]
                break

    text = data.decode("utf-8", errors="replace")
    lines = text.splitlines()
    return "\n".join(lines[-max_lines:])[-max_chars:]


def is_process_alive(pid: int) -> bool:
    if pid <= 0:
        return False
    try:
        os.kill(pid, 0)
        return True
    except OSError:
        return False


def stop_process_group(pid: int) -> None:
    try:
        pgid = os.getpgid(pid)
    except OSError:
        return

    try:
        os.killpg(pgid, signal.SIGTERM)
    except OSError:
        return


def http_probe(url: str, timeout: float = 0.5) -> Tuple[bool, str]:
    try:
        request = urllib.request.Request(
            url,
            headers={"User-Agent": "quickstart-tui/0.2"},
        )
        with urllib.request.urlopen(request, timeout=timeout) as response:
            code = getattr(response, "status", None) or response.getcode()
            return True, f"HTTP {code}"
    except urllib.error.HTTPError as exc:
        return exc.code < 500, f"HTTP {exc.code}"
    except Exception as exc:  # noqa: BLE001
        return False, str(exc)


def port_probe(host: str, port: int, timeout: float = 0.35) -> bool:
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except OSError:
        return False


def command_available(binary: str) -> bool:
    return shutil.which(binary) is not None


class QuickstartApp:
    def __init__(self, config_path: Path) -> None:
        self.config_path = config_path.resolve()
        self.root_dir = self.config_path.parent.parent.resolve()
        self.config = load_json(self.config_path)
        self.language_section = {"title": "Language", "title_ja": "言語"}
        self.language_item_ids = ["__language_ja", "__language_en"]
        self.validate_section = {"title": "Validate Mode", "title_ja": "Validate モード"}
        self.validate_item_ids = ["__validate_relaxed", "__validate_strict"]
        self.items = self._build_items()
        self.sections = self.config.get("sections", [])
        self.state_path = self.root_dir / self.config.get(
            "state_path", "launcher/.state/state.json"
        )
        self.logs_dir = self.root_dir / self.config.get(
            "logs_dir", "launcher/.state/logs"
        )
        self.lock = threading.RLock()
        self.processes: Dict[str, subprocess.Popen[Any]] = {}
        self.workflow_threads: Dict[str, threading.Thread] = {}
        self.service_threads: Dict[str, threading.Thread] = {}
        self.selection = 0
        self._docker_cache: Dict[str, Tuple[float, Dict[str, str]]] = {}
        self.state = self._load_state()
        self.current_db = self.state.get("db_mode") or self.config.get(
            "default_db", "postgres"
        )
        if self.current_db not in self.config.get("db_modes", {}):
            self.current_db = next(iter(self.config.get("db_modes", {"postgres": {}})))
        self.current_language = self._normalize_language(
            self.state.get("language") or self.config.get("default_language", "en")
        )
        self.validate_strict = bool(
            self.state.get("validate_strict", self.config.get("default_validate_strict", False))
        )
        self.state["db_mode"] = self.current_db
        self.state["language"] = self.current_language
        self.state["validate_strict"] = self.validate_strict
        self.message = self.t("ready")
        self._cleanup_restored_state()
        self.rows = self._build_rows()
        self.selection = self._default_selection_index()
        self._coerce_selection()

    def _build_items(self) -> Dict[str, Dict[str, Any]]:
        items = {
            "__language_ja": {
                "id": "__language_ja",
                "kind": "language",
                "language_code": "ja",
                "label": "Japanese",
                "label_ja": "日本語",
                "description": "Switch the display language to Japanese.",
                "description_ja": "表示言語を日本語に切り替えます。",
                "detail_lines": [
                    "Use the arrow keys to move here first, then press Enter to switch."
                ],
                "detail_lines_ja": [
                    "起動したらまずここに矢印で移動して、Enter で切り替えられます。"
                ],
            },
            "__language_en": {
                "id": "__language_en",
                "kind": "language",
                "language_code": "en",
                "label": "English",
                "label_ja": "English",
                "description": "Switch the display language to English.",
                "description_ja": "表示言語を英語に切り替えます。",
                "detail_lines": [
                    "Use the arrow keys to move here first, then press Enter to switch."
                ],
                "detail_lines_ja": [
                    "起動したらまずここに矢印で移動して、Enter で切り替えられます。"
                ],
            },
            "__validate_relaxed": {
                "id": "__validate_relaxed",
                "kind": "validate_mode",
                "validate_strict": False,
                "label": "Standard",
                "label_ja": "通常",
                "description": "Warnings keep the run green. Use this when you want the fastest path to API and sample web.",
                "description_ja": "warning はそのまま通します。API と sample web まで最短で進みたいとき向けです。",
                "detail_lines": [
                    "Quick Start runs validate-ingest without --strict in this mode."
                ],
                "detail_lines_ja": [
                    "このモードでは Quick Start が validate-ingest を --strict なしで実行します。"
                ],
            },
            "__validate_strict": {
                "id": "__validate_strict",
                "kind": "validate_mode",
                "validate_strict": True,
                "label": "Strict",
                "label_ja": "Strict",
                "description": "Warnings fail the run as well. Use this when you want the launcher to stop before API and sample web.",
                "description_ja": "warning も failure 扱いにします。API や sample web の前でしっかり止めたいとき向けです。",
                "detail_lines": [
                    "Quick Start adds --strict to validate-ingest in this mode."
                ],
                "detail_lines_ja": [
                    "このモードでは Quick Start が validate-ingest に --strict を付けます。"
                ],
            },
        }
        items.update({item["id"]: item for item in self.config.get("items", [])})
        return items

    def _normalize_language(self, language: str) -> str:
        return language if language in UI_STRINGS else "en"

    def _language_name(self, language: Optional[str] = None) -> str:
        language = self._normalize_language(language or self.current_language)
        return LANGUAGE_NAMES.get(language, LANGUAGE_NAMES["en"])[self.current_language]

    def _language_selector(self) -> str:
        japanese = "[Japanese]" if self.current_language == "ja" else "Japanese"
        english = "[English]" if self.current_language == "en" else "English"
        return self.t("language_bar", japanese=japanese, english=english)

    def t(self, key: str, **kwargs: Any) -> str:
        current = UI_STRINGS.get(self.current_language, UI_STRINGS["en"])
        template = current.get(key, UI_STRINGS["en"].get(key, key))
        return template.format(**kwargs)

    def _localized_text(self, source: Dict[str, Any], field: str) -> str:
        if self.current_language != "en":
            localized = source.get(f"{field}_{self.current_language}")
            if isinstance(localized, str):
                return localized
        value = source.get(field, "")
        return value if isinstance(value, str) else ""

    def _localized_lines(self, source: Dict[str, Any], field: str) -> List[str]:
        if self.current_language != "en":
            localized = source.get(f"{field}_{self.current_language}")
            if isinstance(localized, list):
                return [str(line) for line in localized]
        value = source.get(field, [])
        if isinstance(value, list):
            return [str(line) for line in value]
        return []

    def _project_name(self) -> str:
        return self._localized_text(self.config, "project_name") or self.config.get(
            "project_name", "project"
        )

    def _section_title(self, section: Dict[str, Any]) -> str:
        return self._localized_text(section, "title") or section.get("title", "")

    def _db_mode_label(self, mode: str) -> str:
        return self.config.get("db_modes", {}).get(mode, {}).get("label", mode)

    def _validate_mode_label(self) -> str:
        item_id = "__validate_strict" if self.validate_strict else "__validate_relaxed"
        return self._display_label(self.items[item_id], include_order=False)

    def _display_label(self, item: Dict[str, Any], include_order: bool = True) -> str:
        label = self._localized_text(item, "label") or item["id"]
        quick_order = item.get("quick_order")
        if include_order and quick_order is not None:
            return f"{quick_order}. {label}"
        return label

    def _display_status_note(self, note: Optional[str]) -> str:
        mapping = {
            None: self.t("unknown"),
            "running": self.t("running"),
            "finished": self.t("finished"),
            "failed": self.t("failed"),
            "skipped": self.t("skipped"),
            "stopped": self.t("stopped"),
            "stopping": self.t("stopping"),
            "canceled": self.t("canceled"),
            "detached session ended": self.t("detached_ended"),
        }
        return mapping.get(note, note or self.t("unknown"))

    def _quick_tips(self) -> List[str]:
        return self._render_lines(self._localized_lines(self.config, "quick_tips"))

    def _load_state(self) -> Dict[str, Any]:
        if not self.state_path.is_file():
            return {
                "db_mode": self.config.get("default_db", "postgres"),
                "language": self.config.get("default_language", "en"),
                "items": {},
            }

        try:
            payload = load_json(self.state_path)
        except Exception:  # noqa: BLE001
            return {
                "db_mode": self.config.get("default_db", "postgres"),
                "language": self.config.get("default_language", "en"),
                "items": {},
            }

        payload.setdefault("items", {})
        return payload

    def _save_state(self) -> None:
        with self.lock:
            save_json(self.state_path, self.state)

    def _cleanup_restored_state(self) -> None:
        for item_id, item_state in self.state.get("items", {}).items():
            pid = item_state.get("pid")
            if pid and not is_process_alive(pid):
                item_state["pid"] = None
                item_state.setdefault("last_exit_code", None)
                item_state["status_note"] = "detached session ended"
                item_state["finished_at"] = now_iso()
            elif item_state.get("status_note") == "running" and not pid:
                item_state["status_note"] = "detached session ended"
                item_state["finished_at"] = now_iso()
        self._save_state()

    def _build_rows(self) -> List[Dict[str, Any]]:
        rows: List[Dict[str, Any]] = [{"kind": "header", "section": self.language_section}]
        for idx, item_id in enumerate(self.language_item_ids):
            rows.append(
                {
                    "kind": "item",
                    "item_id": item_id,
                    "item": self.items[item_id],
                    "tree_prefix": "`- " if idx == len(self.language_item_ids) - 1 else "|- ",
                }
            )
        rows.append({"kind": "header", "section": self.validate_section})
        for idx, item_id in enumerate(self.validate_item_ids):
            rows.append(
                {
                    "kind": "item",
                    "item_id": item_id,
                    "item": self.items[item_id],
                    "tree_prefix": "`- " if idx == len(self.validate_item_ids) - 1 else "|- ",
                }
            )
        for section in self.sections:
            rows.append({"kind": "header", "section": section})
            item_ids = section.get("item_ids", [])
            for idx, item_id in enumerate(item_ids):
                item = self.items.get(item_id)
                if item is None:
                    continue
                rows.append(
                    {
                        "kind": "item",
                        "item_id": item_id,
                        "item": item,
                        "tree_prefix": "`- " if idx == len(item_ids) - 1 else "|- ",
                    }
                )
        return rows

    def _default_selection_index(self) -> int:
        for idx, row in enumerate(self.rows):
            if row["kind"] != "item":
                continue
            item = row["item"]
            if item.get("kind") == "language" and item.get("language_code") == self.current_language:
                return idx
        selectable = self._selectable_indices()
        return selectable[0] if selectable else 0

    def _selectable_indices(self) -> List[int]:
        return [idx for idx, row in enumerate(self.rows) if row["kind"] == "item"]

    def _coerce_selection(self) -> None:
        selectable = self._selectable_indices()
        if not selectable:
            self.selection = 0
            return
        if self.selection not in selectable:
            self.selection = selectable[0]

    def _item_state(self, item_id: str) -> Dict[str, Any]:
        items = self.state.setdefault("items", {})
        return items.setdefault(item_id, {})

    def _db_context(self) -> Dict[str, str]:
        mode = self.config.get("db_modes", {}).get(self.current_db, {})
        placeholders = dict(mode.get("placeholders", {}))
        placeholders.setdefault("db", self.current_db)
        placeholders.setdefault("db_label", mode.get("label", self.current_db))
        placeholders.setdefault("validate_flags", " --strict" if self.validate_strict else "")
        placeholders.setdefault("validate_mode_label", self._validate_mode_label())
        return placeholders

    def _render_template(self, template: str) -> str:
        return template.format_map(self._db_context())

    def _render_lines(self, lines: List[str]) -> List[str]:
        return [self._render_template(line) for line in lines]

    def _resolved_path(self, value: str) -> Path:
        return (self.root_dir / value).resolve()

    def _needs_setup(self, item: Dict[str, Any]) -> bool:
        paths = item.get("setup_if_missing_paths") or []
        if not paths:
            return False
        return not all(self._resolved_path(path).exists() for path in paths)

    def _current_env_db(self) -> str:
        env_candidates = self.config.get("env_files", [])
        for relative in env_candidates:
            env_path = self.root_dir / relative
            if not env_path.is_file():
                continue
            for line in env_path.read_text(encoding="utf-8").splitlines():
                if line.startswith("DATABASE_TYPE="):
                    return line.split("=", 1)[1].strip() or self.t("unknown")
        return self.t("unknown")

    def _log_path(self, item_id: str) -> Path:
        stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        safe_item_id = item_id.replace("/", "_")
        path = self.logs_dir / safe_item_id / f"{stamp}.log"
        path.parent.mkdir(parents=True, exist_ok=True)
        return path

    def _command_spec(
        self, item: Dict[str, Any], action: str = "command"
    ) -> Tuple[List[str], Path]:
        command = item.get(action)
        if not command:
            raise ValueError(f"item {item['id']} does not define {action}")

        command = self._render_template(command)
        runner = item.get("runner", "direct")
        item_cwd = item.get("cwd")
        resolved_cwd = self._resolved_path(item_cwd) if item_cwd else self.root_dir

        entrypoint = None
        try:
            parts = shlex.split(command)
            entrypoint = parts[0] if parts else None
        except ValueError:
            entrypoint = None

        if runner == "dev_shell" and entrypoint and command_available(entrypoint):
            return ["bash", "-lc", command], resolved_cwd

        if runner == "dev_shell" and command_available("nix"):
            inner = command
            if item_cwd:
                inner = f"cd {shlex.quote(item_cwd)} && {command}"
                resolved_cwd = self.root_dir
            argv = ["nix", "develop", "--command", "bash", "-lc", inner]
            return argv, resolved_cwd

        return ["bash", "-lc", command], resolved_cwd

    def _spawn_managed_command(
        self, item: Dict[str, Any], action: str = "command"
    ) -> Optional[Path]:
        item_id = item["id"]
        item_state = self._item_state(item_id)
        pid = item_state.get("pid")
        label = self._display_label(item, include_order=False)
        if pid and is_process_alive(pid):
            self.message = self.t("already_running", label=label)
            return None

        log_path = self._log_path(item_id)
        argv, cwd = self._command_spec(item, action=action)

        log_handle = log_path.open("w", encoding="utf-8")
        log_handle.write(f"$ {' '.join(shlex.quote(part) for part in argv)}\n")
        log_handle.write(f"# cwd: {cwd}\n")
        log_handle.write(f"# started_at: {now_iso()}\n\n")
        log_handle.flush()

        process = subprocess.Popen(
            argv,
            cwd=str(cwd),
            stdout=log_handle,
            stderr=subprocess.STDOUT,
            text=True,
            preexec_fn=os.setsid,
        )
        log_handle.close()

        self.processes[item_id] = process
        item_state.update(
            {
                "pid": process.pid,
                "log_path": str(log_path.relative_to(self.root_dir)),
                "started_at": now_iso(),
                "finished_at": None,
                "last_exit_code": None,
                "status_note": "running",
            }
        )
        self._save_state()
        self.message = self.t("started", label=label)
        return log_path

    def _stop_managed_item(self, item: Dict[str, Any]) -> None:
        item_state = self._item_state(item["id"])
        label = self._display_label(item, include_order=False)
        pid = item_state.get("pid")
        if not pid:
            self.message = self.t("not_managed", label=label)
            return
        stop_process_group(pid)
        item_state["status_note"] = "stopping"
        self._save_state()
        self.message = self.t("stopping_item", label=label)

    def _start_item(
        self,
        item_id: str,
        from_workflow: bool = False,
        workflow_item_id: Optional[str] = None,
    ) -> bool:
        item = self.items[item_id]
        kind = item["kind"]
        label = self._display_label(item, include_order=False)

        if kind == "language":
            self._set_language(item["language_code"])
            return True
        if kind == "validate_mode":
            self._set_validate_strict(bool(item.get("validate_strict")))
            return True
        if kind == "link":
            self._open_item_url(item)
            return True
        if kind == "workflow":
            return self._start_workflow(item)
        if kind == "docker":
            docker_status = self._docker_status(item)
            if docker_status["status"] == "running":
                self.message = self.t("already_running", label=label)
                return True
            self._spawn_managed_command(item, action="up")
            return True
        if kind == "service":
            status_code, _ = self._item_status(item)
            if status_code in {"RUN", "EXT"}:
                self.message = self.t("already_running", label=label)
                return True
            if self._needs_setup(item):
                setup_task_id = item.get("setup_task_id")
                if setup_task_id:
                    if from_workflow:
                        if not self._run_and_wait_task(
                            setup_task_id,
                            workflow_item_id=workflow_item_id,
                        ):
                            return False
                    else:
                        return self._start_service_with_setup_async(
                            item,
                            workflow_item_id=workflow_item_id,
                        )
            self._spawn_managed_command(item, action="command")
            return True
        if kind == "task":
            if item.get("skip_if_exists_paths") and all(
                self._resolved_path(path).exists() for path in item["skip_if_exists_paths"]
            ):
                state = self._item_state(item_id)
                log_path = self._log_path(item_id)
                with log_path.open("w", encoding="utf-8") as handle:
                    handle.write("Skipped because required paths already exist.\n")
                state.update(
                    {
                        "log_path": str(log_path.relative_to(self.root_dir)),
                        "started_at": now_iso(),
                        "finished_at": now_iso(),
                        "last_exit_code": 0,
                        "status_note": "skipped",
                    }
                )
                self._save_state()
                self.message = self.t("skipped_item", label=label)
                return True
            self._spawn_managed_command(item, action="command")
            return True
        return False

    def _workflow_cancel_requested(self, workflow_item_id: Optional[str]) -> bool:
        if not workflow_item_id:
            return False
        return bool(self._item_state(workflow_item_id).get("cancel_requested"))

    def _run_and_wait_task(
        self,
        item_id: str,
        workflow_item_id: Optional[str] = None,
        already_started: bool = False,
    ) -> bool:
        if not already_started:
            self._start_item(item_id, from_workflow=True)
        item_state = self._item_state(item_id)
        pid = item_state.get("pid")
        if not pid:
            return item_state.get("last_exit_code", 1) == 0

        while True:
            self._reconcile_processes()
            if self._workflow_cancel_requested(workflow_item_id):
                item = self.items.get(item_id)
                if item and item["kind"] == "task":
                    self._stop_managed_item(item)
                return False
            state = self._item_state(item_id)
            current_pid = state.get("pid")
            if not current_pid:
                return state.get("last_exit_code", 1) == 0
            time.sleep(0.25)

    def _start_service_with_setup_async(
        self, item: Dict[str, Any], workflow_item_id: Optional[str] = None
    ) -> bool:
        item_id = item["id"]
        label = self._display_label(item, include_order=False)
        thread = self.service_threads.get(item_id)
        if thread and thread.is_alive():
            self.message = self.t("already_running", label=label)
            return True

        cancel_scope_id = workflow_item_id or item_id
        setup_task_id = item.get("setup_task_id")
        service_state = self._item_state(item_id)
        service_state.update(
            {
                "started_at": now_iso(),
                "finished_at": None,
                "last_exit_code": None,
                "status_note": "starting",
                "cancel_requested": False,
                "current_step": setup_task_id,
                "current_step_started_at": now_iso() if setup_task_id else None,
            }
        )
        self._save_state()

        def runner() -> None:
            exit_code = 0
            canceled = False
            try:
                if setup_task_id and self._needs_setup(item):
                    ok = self._run_and_wait_task(
                        setup_task_id,
                        workflow_item_id=cancel_scope_id,
                    )
                    if not ok:
                        canceled = self._workflow_cancel_requested(cancel_scope_id)
                        setup_state = self._item_state(setup_task_id)
                        exit_code = setup_state.get("last_exit_code")
                        if exit_code is None:
                            exit_code = 130 if canceled else 1
                        return

                if self._workflow_cancel_requested(cancel_scope_id):
                    canceled = True
                    exit_code = 130
                    return

                self._spawn_managed_command(item, action="command")
            except Exception:  # noqa: BLE001
                exit_code = 1
                service_state["status_note"] = "failed"
            finally:
                service_state["current_step"] = None
                service_state["current_step_started_at"] = None
                service_state["cancel_requested"] = False
                if canceled:
                    service_state["finished_at"] = now_iso()
                    service_state["last_exit_code"] = 130
                    service_state["status_note"] = "canceled"
                elif exit_code != 0:
                    service_state["finished_at"] = now_iso()
                    service_state["last_exit_code"] = exit_code
                    if service_state.get("status_note") == "stopping":
                        service_state["status_note"] = "stopped"
                    elif service_state.get("status_note") != "failed":
                        service_state["status_note"] = "failed"
                self._save_state()

        thread = threading.Thread(target=runner, daemon=True)
        self.service_threads[item_id] = thread
        thread.start()
        self.message = self.t("started", label=label)
        return True

    def _wait_for_service_ready(
        self, item: Dict[str, Any], workflow_item_id: Optional[str] = None
    ) -> bool:
        timeout = item.get("ready_timeout_seconds", 20)
        health_url = item.get("health_url")
        port = item.get("port")
        host = item.get("host", "127.0.0.1")
        started = time.time()
        while time.time() - started < timeout:
            if self._workflow_cancel_requested(workflow_item_id):
                return False
            if health_url:
                ok, _ = http_probe(self._render_template(health_url))
                if ok:
                    return True
            elif port and port_probe(host, int(port)):
                return True
            elif not health_url and not port:
                return True
            time.sleep(0.5)
        return False

    def _start_workflow(self, item: Dict[str, Any]) -> bool:
        item_id = item["id"]
        label = self._display_label(item, include_order=False)
        thread = self.workflow_threads.get(item_id)
        if thread and thread.is_alive():
            self.message = self.t("already_running", label=label)
            return False

        def runner() -> None:
            workflow_state = self._item_state(item_id)
            log_path = self._log_path(item_id)
            workflow_state.update(
                {
                    "log_path": str(log_path.relative_to(self.root_dir)),
                    "started_at": now_iso(),
                    "finished_at": None,
                    "last_exit_code": None,
                    "status_note": "running",
                    "cancel_requested": False,
                    "current_step": None,
                    "current_step_started_at": None,
                }
            )
            self._save_state()

            exit_code = 0
            canceled = False
            with log_path.open("w", encoding="utf-8") as handle:
                handle.write(f"# workflow: {label}\n")
                handle.write(f"# started_at: {now_iso()}\n\n")
                handle.flush()

                try:
                    for step_id in item.get("steps", []):
                        if self._workflow_cancel_requested(item_id):
                            canceled = True
                            handle.write(f"{self.t('workflow_canceled', label=label)}\n")
                            break
                        step = self.items[step_id]
                        step_label = self._display_label(step, include_order=False)
                        workflow_state["current_step"] = step_id
                        workflow_state["current_step_started_at"] = now_iso()
                        self._save_state()
                        handle.write(f"== {step_label} ==\n")
                        handle.flush()

                        ok = self._start_item(
                            step_id,
                            from_workflow=True,
                            workflow_item_id=item_id,
                        )
                        if not ok:
                            if self._workflow_cancel_requested(item_id):
                                canceled = True
                                handle.write(f"{self.t('workflow_canceled', label=label)}\n")
                                break
                            exit_code = 1
                            handle.write(f"{self.t('failed_to_start_step')}\n")
                            break

                        if step["kind"] in {"task", "docker"}:
                            if not self._run_and_wait_task(
                                step_id,
                                workflow_item_id=item_id,
                                already_started=True,
                            ):
                                if self._workflow_cancel_requested(item_id):
                                    canceled = True
                                    handle.write(
                                        f"{self.t('workflow_canceled', label=label)}\n"
                                    )
                                    break
                                exit_code = 1
                                handle.write(f"{self.t('failed')}: {step_label}\n")
                                break
                        elif step["kind"] == "service":
                            if not self._wait_for_service_ready(step, workflow_item_id=item_id):
                                if self._workflow_cancel_requested(item_id):
                                    canceled = True
                                    handle.write(
                                        f"{self.t('workflow_canceled', label=label)}\n"
                                    )
                                    break
                                exit_code = 1
                                handle.write(
                                    f"{self.t('service_not_ready', label=step_label)}\n"
                                )
                                break
                        handle.write("ok\n\n")
                        handle.flush()
                except Exception:  # noqa: BLE001
                    exit_code = 1
                    handle.write(traceback.format_exc())

            workflow_state["current_step"] = None
            workflow_state["current_step_started_at"] = None
            workflow_state["cancel_requested"] = False
            workflow_state["finished_at"] = now_iso()
            workflow_state["last_exit_code"] = 130 if canceled else exit_code
            workflow_state["status_note"] = (
                "canceled" if canceled else "finished" if exit_code == 0 else "failed"
            )
            self._save_state()
            self.message = (
                self.t("workflow_canceled", label=label)
                if canceled
                else self.t("workflow_finished", label=label)
                if exit_code == 0
                else self.t("workflow_failed", label=label)
            )

        thread = threading.Thread(target=runner, daemon=True)
        self.workflow_threads[item_id] = thread
        thread.start()
        self.message = self.t("started", label=label)
        return True

    def _stop_item(self, item_id: str) -> None:
        item = self.items[item_id]
        label = self._display_label(item, include_order=False)
        if item["kind"] == "workflow":
            thread = self.workflow_threads.get(item_id)
            if not thread or not thread.is_alive():
                self.message = self.t("not_managed", label=label)
                return
            workflow_state = self._item_state(item_id)
            workflow_state["cancel_requested"] = True
            workflow_state["status_note"] = "stopping"
            current_step_id = workflow_state.get("current_step")
            current_step = self.items.get(current_step_id) if current_step_id else None
            if current_step:
                if current_step["kind"] == "docker":
                    self._spawn_managed_command(current_step, action="down")
                elif current_step["kind"] in {"service", "task"}:
                    self._stop_managed_item(current_step)
            self._save_state()
            self.message = self.t("workflow_canceling", label=label)
            return
        if item["kind"] == "service":
            service_thread = self.service_threads.get(item_id)
            service_state = self._item_state(item_id)
            if service_thread and service_thread.is_alive():
                service_state["cancel_requested"] = True
                service_state["status_note"] = "stopping"
                current_step_id = service_state.get("current_step")
                current_step = self.items.get(current_step_id) if current_step_id else None
                if current_step and current_step["kind"] == "task":
                    self._stop_managed_item(current_step)
                elif service_state.get("pid"):
                    self._stop_managed_item(item)
                self._save_state()
                self.message = self.t("stopping_item", label=label)
                return
            self._stop_managed_item(item)
            return
        if item["kind"] == "docker":
            self._spawn_managed_command(item, action="down")
            return
        if item["kind"] == "task":
            self._stop_managed_item(item)
            return
        self.message = self.t("stop_not_supported", label=label)

    def _restart_item(self, item_id: str) -> None:
        item = self.items[item_id]
        if item["kind"] == "docker":
            self._spawn_managed_command(item, action="down")
            time.sleep(0.2)
            self._spawn_managed_command(item, action="up")
            return
        if item["kind"] == "service":
            self._stop_managed_item(item)
            time.sleep(0.2)
            self._start_item(item_id)
            return
        self.message = self.t("restart_services_only")

    def _open_item_url(self, item: Dict[str, Any]) -> None:
        label = self._display_label(item, include_order=False)
        url = item.get("url")
        if not url:
            self.message = self.t("no_url", label=label)
            return
        port = item.get("port")
        host = item.get("host", "127.0.0.1")
        if port and not port_probe(host, int(port)):
            self.message = self.t("not_ready_yet", label=label)
            return
        url = self._render_template(url)
        try:
            if sys.platform == "darwin":
                subprocess.Popen(["open", url])
            elif os.name == "nt":
                subprocess.Popen(["cmd", "/c", "start", "", url])
            else:
                subprocess.Popen(["xdg-open", url])
            self.message = self.t("opened_url", url=url)
        except Exception as exc:  # noqa: BLE001
            self.message = self.t("open_failed", error=exc)

    def _docker_status(self, item: Dict[str, Any]) -> Dict[str, str]:
        container = item.get("container_name")
        if not container:
            return {
                "status": "unknown",
                "detail": self.t("no_container_configured"),
            }

        cached = self._docker_cache.get(container)
        if cached and time.time() - cached[0] < 1.0:
            return cached[1]

        try:
            completed = subprocess.run(
                [
                    "docker",
                    "inspect",
                    "--format",
                    "{{.State.Status}}|{{if .State.Health}}{{.State.Health.Status}}{{end}}",
                    container,
                ],
                cwd=str(self.root_dir),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                check=False,
                timeout=1.5,
            )
        except Exception as exc:  # noqa: BLE001
            result = {"status": "error", "detail": str(exc)}
            self._docker_cache[container] = (time.time(), result)
            return result

        if completed.returncode != 0:
            result = {"status": "stopped", "detail": self.t("container_not_created")}
            self._docker_cache[container] = (time.time(), result)
            return result

        status, _, health = completed.stdout.strip().partition("|")
        detail = health or status or self.t("unknown")
        result = {"status": status or self.t("unknown"), "detail": detail}
        self._docker_cache[container] = (time.time(), result)
        return result

    def _item_status(self, item: Dict[str, Any]) -> Tuple[str, str]:
        state = self._item_state(item["id"])
        pid = state.get("pid")
        kind = item["kind"]

        if kind == "workflow":
            thread = self.workflow_threads.get(item["id"])
            if thread and thread.is_alive():
                if state.get("cancel_requested"):
                    return "STOP", self.t("stopping")
                step_id = state.get("current_step")
                step_item = self.items.get(step_id) if step_id else None
                step_label = (
                    self._display_label(step_item, include_order=False)
                    if step_item
                    else "..."
                )
                step_elapsed = format_elapsed(
                    elapsed_seconds_since(state.get("current_step_started_at"))
                )
                return "RUN", f"{self.t('workflow_step', step=step_label)} / {step_elapsed}"
            exit_code = state.get("last_exit_code")
            if state.get("status_note") == "canceled":
                return "STOP", self._display_status_note(state.get("status_note"))
            if exit_code == 0:
                return "OK", self._display_status_note(state.get("status_note"))
            if exit_code is None:
                return "--", self.t("not_started")
            return "ERR", self._display_status_note(state.get("status_note"))

        if kind == "language":
            if item.get("language_code") == self.current_language:
                return "ON", self.t("selected")
            return "--", self._language_name(item.get("language_code"))

        if kind == "validate_mode":
            if bool(item.get("validate_strict")) == self.validate_strict:
                return "ON", self.t("selected")
            return "--", self._validate_mode_label()

        if kind == "task":
            if pid and is_process_alive(pid):
                return "RUN", self.t("pid", pid=pid)
            exit_code = state.get("last_exit_code")
            reported_status = self._reported_status_from_log(item)
            if exit_code is None:
                return "--", self.t("not_started")
            if reported_status:
                normalized = reported_status.lower()
                detail = f"{item['report_status_prefix']} {reported_status}"
                if exit_code == 0:
                    if normalized == "warning":
                        return "WARN", detail
                    if normalized == "failed":
                        return "ERR", detail
                    if normalized == "ok":
                        return "OK", detail
                return "ERR", detail
            if exit_code == 0:
                return "OK", self._display_status_note(state.get("status_note"))
            return "ERR", f"exit {exit_code}"

        if kind == "service":
            service_thread = self.service_threads.get(item["id"])
            if service_thread and service_thread.is_alive():
                if state.get("cancel_requested") or state.get("status_note") == "stopping":
                    return "STOP", self.t("stopping")
                current_step = state.get("current_step")
                if current_step:
                    step_item = self.items.get(current_step)
                    step_label = (
                        self._display_label(step_item, include_order=False)
                        if step_item
                        else current_step
                    )
                    step_elapsed = format_elapsed(
                        elapsed_seconds_since(state.get("current_step_started_at"))
                    )
                    return (
                        "BOOT",
                        f"{self.t('service_setup_step', step=step_label)} / {step_elapsed}",
                    )
                return "BOOT", self.t("starting")
            url = item.get("health_url")
            port = item.get("port")
            host = item.get("host", "127.0.0.1")
            externally_running = False
            detail = self.t("stopped")
            if url:
                externally_running, detail = http_probe(self._render_template(url))
            elif port:
                externally_running = port_probe(host, int(port))
                detail = (
                    self.t("port_open", port=port)
                    if externally_running
                    else self.t("port_closed", port=port)
                )

            if pid and is_process_alive(pid):
                if externally_running:
                    return "RUN", detail
                return "BOOT", detail if port else self.t("pid", pid=pid)
            if externally_running:
                return "EXT", detail
            return "--", self.t("stopped")

        if kind == "docker":
            docker_state = self._docker_status(item)
            status = docker_state["status"]
            if status == "running":
                return "RUN", docker_state["detail"]
            if status == "error":
                return "ERR", docker_state["detail"]
            return "--", docker_state["detail"]

        if kind == "link":
            port = item.get("port")
            host = item.get("host", "127.0.0.1")
            if port:
                ok = port_probe(host, int(port))
                detail = (
                    self.t("port_open", port=port)
                    if ok
                    else self.t("port_closed", port=port)
                )
                return ("UP" if ok else "--"), detail
            url = self._render_template(item["url"])
            ok, detail = http_probe(url)
            return ("UP" if ok else "--"), detail

        return "??", self.t("unknown")

    def _selected_item(self) -> Optional[Dict[str, Any]]:
        if not self.rows:
            return None
        row = self.rows[self.selection]
        if row["kind"] != "item":
            return None
        return row["item"]

    def _selected_item_logs(self, item: Dict[str, Any]) -> str:
        state = self._item_state(item["id"])
        log_path_value = state.get("log_path")
        if log_path_value:
            text = read_tail(self.root_dir / log_path_value)
            if text:
                return text

        if item["kind"] == "docker":
            container = item.get("container_name")
            if not container:
                return ""
            try:
                completed = subprocess.run(
                    ["docker", "logs", "--tail", "60", container],
                    cwd=str(self.root_dir),
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    check=False,
                    timeout=1.5,
                )
                return completed.stdout.strip()
            except Exception as exc:  # noqa: BLE001
                return self.t("docker_logs_failed", error=exc)

        if item["kind"] == "service":
            current_step_id = state.get("current_step")
            if current_step_id:
                current_step_state = self._item_state(current_step_id)
                current_log_path = current_step_state.get("log_path")
                if current_log_path:
                    text = read_tail(self.root_dir / current_log_path)
                    if text:
                        return text
            return self.t("logs_service_hint")

        return ""

    def _reported_status_from_log(self, item: Dict[str, Any]) -> Optional[str]:
        prefix = item.get("report_status_prefix")
        if not prefix:
            return None

        state = self._item_state(item["id"])
        log_path_value = state.get("log_path")
        if not log_path_value:
            return None

        text = read_tail(self.root_dir / log_path_value, max_lines=120)
        for line in reversed(text.splitlines()):
            stripped = line.strip()
            if stripped.startswith(prefix):
                return stripped[len(prefix) :].strip()
        return None

    def _detail_lines(self, item: Dict[str, Any], width: int) -> List[str]:
        status_code, status_detail = self._item_status(item)
        lines: List[str] = []

        quick_tips = self._quick_tips()
        if quick_tips:
            lines.append(self.t("quick_path"))
            for idx, tip in enumerate(quick_tips, start=1):
                lines.append(f"{idx}. {tip}")
            lines.append("")

        lines.extend(
            [
                self._display_label(item, include_order=False),
                f"{self.t('kind')}: {item['kind']}",
                f"{self.t('status')}: {status_code} {status_detail}",
            ]
        )

        quick_order = item.get("quick_order")
        if quick_order is not None:
            lines.append(f"{self.t('quick_order')}: {quick_order}")

        description = self._localized_text(item, "description")
        if description:
            lines.append("")
            lines.extend(self._render_lines([description]))

        if item.get("url"):
            lines.append("")
            lines.append(f"{self.t('url')}: {self._render_template(item['url'])}")

        if item.get("health_url"):
            lines.append(f"{self.t('health')}: {self._render_template(item['health_url'])}")

        detail_lines = self._localized_lines(item, "detail_lines")
        if detail_lines:
            lines.append("")
            lines.extend(self._render_lines(detail_lines))

        if item["kind"] in {"workflow", "service"}:
            current_state = self._item_state(item["id"])
            current_step = current_state.get("current_step")
            if current_step:
                step_item = self.items.get(current_step)
                step_label = (
                    self._display_label(step_item, include_order=False)
                    if step_item
                    else current_step
                )
                step_elapsed = format_elapsed(
                    elapsed_seconds_since(current_state.get("current_step_started_at"))
                )
                lines.append("")
                lines.append(f"{self.t('current_step')}: {step_label}")
                lines.append(f"{self.t('step_elapsed')}: {step_elapsed}")

        state_lines = self._state_detail_lines(item)
        if state_lines:
            lines.append("")
            lines.extend(state_lines)

        lines.append("")
        lines.append(self.t("recent_output"))
        logs = self._selected_item_logs(item) or self.t("no_log_output")
        lines.extend(logs.splitlines())

        wrapped: List[str] = []
        for line in lines:
            wrapped.extend(wrap_text(line, width))
        return wrapped

    def _state_detail_lines(self, item: Dict[str, Any]) -> List[str]:
        state = self._item_state(item["id"])
        started_at = state.get("started_at")
        finished_at = state.get("finished_at")
        last_exit_code = state.get("last_exit_code")

        if started_at is None and finished_at is None and last_exit_code is None:
            return []

        lines = [self.t("last_run")]
        if started_at:
            lines.append(f"{self.t('started_at')}: {started_at}")
        if finished_at:
            lines.append(f"{self.t('finished_at')}: {finished_at}")
        if last_exit_code is not None:
            lines.append(f"{self.t('last_exit_code')}: {last_exit_code}")
        return lines

    def _reconcile_processes(self) -> None:
        for item_id, process in list(self.processes.items()):
            return_code = process.poll()
            if return_code is None:
                continue
            state = self._item_state(item_id)
            state["pid"] = None
            state["last_exit_code"] = return_code
            state["finished_at"] = now_iso()
            if state.get("status_note") == "stopping":
                state["status_note"] = "stopped"
            else:
                state["status_note"] = "finished" if return_code == 0 else "failed"
            self.processes.pop(item_id, None)
            self._save_state()

    def _move_selection(self, delta: int) -> None:
        selectable = self._selectable_indices()
        if not selectable:
            return
        try:
            current_pos = selectable.index(self.selection)
        except ValueError:
            current_pos = 0
        new_pos = max(0, min(len(selectable) - 1, current_pos + delta))
        self.selection = selectable[new_pos]

    def _set_db_mode(self, mode: str) -> None:
        if mode not in self.config.get("db_modes", {}):
            return
        self.current_db = mode
        self.state["db_mode"] = mode
        self._save_state()
        self.message = self.t("db_mode_set", label=self._db_mode_label(mode))

    def _set_validate_strict(self, strict: bool, persist: bool = True) -> None:
        self.validate_strict = strict
        self.state["validate_strict"] = self.validate_strict
        if persist:
            self._save_state()
        self.message = self.t(
            "validate_mode_set", label=self._validate_mode_label()
        )

    def _toggle_validate_strict(self) -> None:
        self._set_validate_strict(not self.validate_strict)

    def _set_language(self, language: str, persist: bool = True) -> None:
        self.current_language = self._normalize_language(language)
        self.state["language"] = self.current_language
        if persist:
            self._save_state()
        self.message = self.t(
            "language_set", language=self._language_name(self.current_language)
        )

    def _toggle_language(self) -> None:
        self._set_language("ja" if self.current_language == "en" else "en")

    def _handle_key(self, key: int) -> bool:
        selected = self._selected_item()

        if key in {ord("q"), 27}:
            return False
        if key in {curses.KEY_UP, ord("k")}:
            self._move_selection(-1)
            return True
        if key in {curses.KEY_DOWN, ord("j")}:
            self._move_selection(1)
            return True
        if key == ord("1"):
            self._set_db_mode("postgres")
            return True
        if key == ord("2"):
            self._set_db_mode("mysql")
            return True
        if key == ord("3"):
            self._set_db_mode("sqlite")
            return True
        if key == ord("l"):
            self._toggle_language()
            return True
        if key == ord("v"):
            self._toggle_validate_strict()
            return True
        if key == 9:
            self._toggle_language()
            return True
        if selected is None:
            return True
        if key in {10, 13, ord("s")}:
            self._start_item(selected["id"])
            return True
        if key == ord("x"):
            self._stop_item(selected["id"])
            return True
        if key == ord("r"):
            self._restart_item(selected["id"])
            return True
        if key == ord("o"):
            self._open_item_url(selected)
            return True
        return True

    def _status_rows(self) -> List[str]:
        lines = []
        for row in self.rows:
            if row["kind"] != "item":
                lines.append(f"# {self._section_title(row['section'])}")
                continue
            item = row["item"]
            code, detail = self._item_status(item)
            lines.append(f"[{code:>4}] {self._display_label(item)}: {detail}")
        return lines

    def print_status(self) -> int:
        self._reconcile_processes()
        print(f"{self._project_name()} {self.t('launcher')}")
        print(self._language_selector())
        print(
            self.t(
                "db_mode_line",
                db_mode=self._db_mode_label(self.current_db),
                env_db=self._current_env_db(),
                validate_mode=self._validate_mode_label(),
                language=self.current_language.upper(),
            )
        )
        quick_tips = self._quick_tips()
        if quick_tips:
            print("")
            print(f"{self.t('quick_path')}:")
            for idx, tip in enumerate(quick_tips, start=1):
                print(f"  {idx}. {tip}")
        print("")
        for line in self._status_rows():
            print(line)
        return 0

    def run(self) -> int:
        if not sys.stdin.isatty() or not sys.stdout.isatty():
            return self.print_status()
        return curses.wrapper(self._run_curses)

    def _run_curses(self, stdscr: Any) -> int:
        curses.curs_set(0)
        stdscr.nodelay(False)
        stdscr.timeout(250)
        if curses.has_colors():
            curses.start_color()
            curses.use_default_colors()
            curses.init_pair(1, curses.COLOR_BLACK, curses.COLOR_CYAN)
            curses.init_pair(2, curses.COLOR_GREEN, -1)
            curses.init_pair(3, curses.COLOR_RED, -1)
            curses.init_pair(4, curses.COLOR_YELLOW, -1)
            curses.init_pair(5, curses.COLOR_BLUE, -1)

        while True:
            self._reconcile_processes()
            self._draw(stdscr)
            key = stdscr.getch()
            if key == -1:
                continue
            if not self._handle_key(key):
                break
        return 0

    def _status_attr(self, code: str) -> int:
        if not curses.has_colors():
            return curses.A_BOLD
        if code in {"OK", "UP", "ON"}:
            return curses.color_pair(2) | curses.A_BOLD
        if code in {"BOOT"}:
            return curses.color_pair(5) | curses.A_BOLD
        if code in {"WARN"}:
            return curses.color_pair(4) | curses.A_BOLD
        if code in {"STOP"}:
            return curses.color_pair(4) | curses.A_BOLD
        if code in {"ERR"}:
            return curses.color_pair(3) | curses.A_BOLD
        if code in {"RUN", "EXT"}:
            return curses.color_pair(4) | curses.A_BOLD
        return curses.color_pair(5) | curses.A_BOLD

    def _draw(self, stdscr: Any) -> None:
        stdscr.erase()
        height, width = stdscr.getmaxyx()
        left_width = min(46, max(30, width // 3))
        right_x = left_width + 2
        right_width = max(20, width - right_x - 1)

        language_bar = self._language_selector()
        header = self.t(
            "header",
            title=self._project_name(),
            launcher=self.t("launcher"),
            current_db=self.current_db,
            validate_mode=self._validate_mode_label(),
            env_db=self._current_env_db(),
        )
        stdscr.addnstr(0, 0, language_bar, width - 1, curses.A_BOLD)
        stdscr.addnstr(1, 0, header, width - 1, curses.A_BOLD)
        stdscr.addnstr(2, 0, self.message, width - 1)

        row_y = 4
        for idx, row in enumerate(self.rows):
            if row_y >= height - 2:
                break
            if row["kind"] == "header":
                stdscr.addnstr(
                    row_y,
                    0,
                    self._section_title(row["section"]),
                    left_width - 1,
                    curses.A_BOLD,
                )
                row_y += 1
                continue

            item = row["item"]
            label = f"{row.get('tree_prefix', '')}{self._display_label(item)}"
            status_code, _ = self._item_status(item)
            attr = curses.A_NORMAL
            if idx == self.selection:
                attr = curses.color_pair(1) if curses.has_colors() else curses.A_REVERSE
            stdscr.addnstr(row_y, 0, f"{label:<30}", left_width - 8, attr)
            stdscr.addnstr(
                row_y,
                max(0, left_width - 7),
                f"{status_code:>4}",
                5,
                attr | self._status_attr(status_code),
            )
            row_y += 1

        for y in range(3, height - 1):
            if left_width < width:
                stdscr.addch(y, left_width, "|")

        selected = self._selected_item()
        detail_lines = (
            self._detail_lines(selected, right_width)
            if selected is not None
            else [self.t("no_item_selected")]
        )
        for idx, line in enumerate(detail_lines[: max(0, height - 4)]):
            stdscr.addnstr(4 + idx, right_x, line, right_width)

        stdscr.addnstr(height - 1, 0, self.t("footer"), width - 1, curses.A_DIM)
        stdscr.refresh()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Local quickstart TUI")
    parser.add_argument(
        "--config",
        default="launcher/station_converter_ja.json",
        help="path to launcher config JSON",
    )
    parser.add_argument(
        "--status",
        action="store_true",
        help="print a text status snapshot instead of opening curses",
    )
    parser.add_argument(
        "--lang",
        choices=["en", "ja"],
        help="override the display language",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    config_path = Path(args.config)
    if not config_path.is_absolute():
        config_path = Path.cwd() / config_path
    app = QuickstartApp(config_path)
    if args.lang:
        app._set_language(args.lang, persist=False)
    if args.status:
        return app.print_status()
    return app.run()


if __name__ == "__main__":
    raise SystemExit(main())
