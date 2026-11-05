import json
from pathlib import Path
import sys
import tempfile
import unittest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from usage_tracker import latest_active_turn, project_skills, read_state, update_state, write_state  # noqa: E402


class UsageTrackerTests(unittest.TestCase):
    def test_skill_usage_is_once_per_turn_and_rule_change_is_hash_based(self):
        state = read_state(Path("missing.json"), "baseline")
        first = update_state(state, "thread", "turn-1", ["project-metrics", "project-metrics"], "baseline")
        self.assertEqual(first["skills"], ["project-metrics"])
        self.assertFalse(first["rule_changed"])
        repeated = update_state(state, "thread", "turn-1", ["autumn-dashboard-api"], "new-hash")
        self.assertEqual(repeated["skills"], ["autumn-dashboard-api", "project-metrics"])
        self.assertTrue(repeated["rule_changed"])
        second = update_state(state, "thread", "turn-2", [], "new-hash")
        self.assertFalse(second["rule_changed"])
        self.assertEqual(len(state["records"]), 2)
        with self.assertRaises(ValueError):
            update_state(state, "thread", "turn-1", [], "baseline")

    def test_state_round_trip_and_current_project_skills(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".agents" / "skills" / "sample-skill").mkdir(parents=True)
            (root / ".agents" / "skills" / "sample-skill" / "SKILL.md").write_text("skill", encoding="utf-8")
            (root / ".agents" / "skills" / "stale-cache").mkdir()
            self.assertEqual(project_skills(root), ["sample-skill"])
            path = root / "state.json"
            state = read_state(path, "original")
            update_state(state, "thread", "turn", ["sample-skill"], "changed")
            write_state(path, state)
            self.assertEqual(read_state(path)["records"][0]["skills"], ["sample-skill"])

    def test_active_turn_comes_from_matching_session_log(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "rollout-thread-a.jsonl"
            events = [
                {"type": "event_msg", "payload": {"type": "task_started", "turn_id": "old"}},
                {"type": "event_msg", "payload": {"type": "task_complete", "turn_id": "old"}},
                {"type": "event_msg", "payload": {"type": "task_started", "turn_id": "new"}},
            ]
            path.write_text("\n".join(json.dumps(item) for item in events) + "\n{partial", encoding="utf-8")
            self.assertEqual(latest_active_turn(Path(directory), "thread-a"), "new")


if __name__ == "__main__":
    unittest.main()
