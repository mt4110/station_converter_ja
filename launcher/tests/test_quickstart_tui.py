import importlib.util
import json
import tempfile
import time
import unittest
import urllib.error
from pathlib import Path
from unittest import mock


MODULE_PATH = Path(__file__).resolve().parents[1] / "quickstart_tui.py"
SPEC = importlib.util.spec_from_file_location("quickstart_tui", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
quickstart_tui = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(quickstart_tui)


def write_test_config(root: Path) -> Path:
    launcher_dir = root / "launcher"
    launcher_dir.mkdir(parents=True, exist_ok=True)
    config_path = launcher_dir / "test_quickstart.json"
    config = {
        "default_db": "postgres",
        "default_language": "en",
        "default_validate_strict": False,
        "logs_dir": "launcher/.state/logs",
        "state_path": "launcher/.state/state.json",
        "db_modes": {"postgres": {"label": "PostgreSQL", "placeholders": {}}},
        "sections": [{"title": "Services", "item_ids": ["docker_item"]}],
        "items": [
            {
                "id": "docker_item",
                "kind": "docker",
                "label": "Docker Item",
                "up": 'python3 -c "import time; time.sleep(5)"',
                "down": 'python3 -c "print(\'down\')"',
            }
        ],
    }
    config_path.write_text(json.dumps(config), encoding="utf-8")
    return config_path


class QuickstartTuiTests(unittest.TestCase):
    def test_http_probe_treats_http_404_as_not_ready(self) -> None:
        error = urllib.error.HTTPError(
            "http://127.0.0.1:3212/health",
            404,
            "Not Found",
            hdrs=None,
            fp=None,
        )
        with mock.patch.object(
            quickstart_tui.urllib.request, "urlopen", side_effect=error
        ):
            ok, detail = quickstart_tui.http_probe("http://127.0.0.1:3212/health")

        self.assertFalse(ok)
        self.assertEqual(detail, "HTTP 404")

    def test_docker_down_can_replace_running_up_command(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config_path = write_test_config(root)
            app = quickstart_tui.QuickstartApp(config_path)
            item = app.items["docker_item"]

            up_log_path = app._spawn_managed_command(item, action="up")
            self.assertIsNotNone(up_log_path)
            up_pid = app._item_state(item["id"]).get("pid")
            self.assertTrue(up_pid)
            self.assertTrue(quickstart_tui.is_process_alive(up_pid))

            down_log_path = app._spawn_managed_command(item, action="down")
            self.assertIsNotNone(down_log_path)

            deadline = time.time() + 3.0
            while time.time() < deadline:
                app._reconcile_processes()
                if app._item_state(item["id"]).get("pid") is None:
                    break
                time.sleep(0.05)

            state = app._item_state(item["id"])
            self.assertIsNone(state.get("pid"))
            self.assertEqual(state.get("last_exit_code"), 0)
            self.assertIn("down", Path(down_log_path).read_text(encoding="utf-8"))

    def test_run_and_wait_task_accepts_externally_running_docker_item(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            config_path = write_test_config(root)
            app = quickstart_tui.QuickstartApp(config_path)

            with mock.patch.object(
                app, "_docker_status", return_value={"status": "running", "detail": "healthy"}
            ):
                self.assertTrue(app._run_and_wait_task("docker_item", already_started=True))


if __name__ == "__main__":
    unittest.main()
