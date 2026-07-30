#!/usr/bin/env python3
"""Run reusable live Umbrello MCP QA scenarios and save transcript artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Any

from mcp_client import EXPECTED_TOOLS, McpClient, McpError, decode_image_content, structured_content


JSON = dict[str, Any]


class SmokeFailure(RuntimeError):
    """A precise assertion or unsupported-target failure in a smoke scenario."""


class Recorder:
    """Deterministically record MCP requests and selected response evidence."""

    def __init__(self) -> None:
        self.entries: list[JSON] = []

    def call(self, client: McpClient, name: str, arguments: JSON | None = None) -> JSON:
        response = client.call_tool(name, arguments)
        self.entries.append(
            {
                "arguments": arguments or {},
                "response": client.transcript_response(name, response),
                "tool": name,
            }
        )
        return response


def snapshot(recorder: Recorder, client: McpClient) -> JSON:
    return structured_content(recorder.call(client, "ui_inspect"))


def target(state: JSON, prefix: str, *, enabled: bool = True) -> JSON:
    targets = state.get("targets", [])
    if not isinstance(targets, list):
        raise SmokeFailure("ui_inspect returned an invalid targets list")
    for item in targets:
        if isinstance(item, dict) and str(item.get("id", "")).startswith(prefix):
            if enabled and not item.get("enabled"):
                continue
            return item
    raise SmokeFailure(f"no {'enabled ' if enabled else ''}target matching {prefix!r}")


def select_and_click(recorder: Recorder, client: McpClient, target_id: str) -> JSON:
    recorder.call(client, "ui_select", {"target_id": target_id})
    return structured_content(recorder.call(client, "ui_click", {}))


def assert_tools(client: McpClient, recorder: Recorder) -> None:
    tools = client.list_tools()
    recorder.entries.append({"response": {"result": {"tools": tools}}, "tool": "tools/list"})
    names = {item.get("name") for item in tools if isinstance(item, dict)}
    if names != EXPECTED_TOOLS:
        raise SmokeFailure(f"expected exactly seven MCP tools {sorted(EXPECTED_TOOLS)}, got {sorted(names)}")


def activate_diagram(recorder: Recorder, client: McpClient, state: JSON) -> JSON:
    if state.get("active_diagram"):
        return state
    diagram = target(state, "diagram:")
    select_and_click(recorder, client, str(diagram["id"]))
    state = snapshot(recorder, client)
    if not state.get("active_diagram"):
        raise SmokeFailure("diagram activation did not produce an active_diagram")
    return state


def sync(recorder: Recorder, client: McpClient, state: JSON) -> JSON:
    revision = state.get("state_revision")
    if not isinstance(revision, int):
        raise SmokeFailure(f"state_revision is not an integer: {revision!r}")
    synced = structured_content(recorder.call(client, "ui_sync", {"after_revision": revision}))
    if synced.get("rendered_revision", -1) < revision:
        raise SmokeFailure(f"ui_sync did not render revision {revision}: {synced!r}")
    return synced


def save_screenshot(recorder: Recorder, client: McpClient, artifact_dir: Path) -> JSON:
    response = recorder.call(client, "ui_screenshot", {})
    png, metadata = decode_image_content(response)
    (artifact_dir / "screenshot.png").write_bytes(png)
    return metadata


def readonly_scenario(client: McpClient, recorder: Recorder, artifact_dir: Path) -> JSON:
    """Exercise a loaded model's native-equivalent gesture, sync, undo, and PNG."""
    assert_tools(client, recorder)
    state = activate_diagram(recorder, client, snapshot(recorder, client))
    node = target(state, "node:")
    recorder.call(client, "ui_select", {"target_id": node["id"]})
    moved = structured_content(
        recorder.call(client, "ui_drag", {"x": 2000.0, "y": 1500.0, "gesture": True})
    )
    undo = target(moved, "history.undo")
    if not undo.get("enabled"):
        raise SmokeFailure("gesture=true drag did not enable Undo")
    synced = sync(recorder, client, moved)
    undone = select_and_click(recorder, client, str(undo["id"]))
    undo_synced = sync(recorder, client, undone)
    after_undo = snapshot(recorder, client)
    redo = target(after_undo, "history.redo")
    if not redo.get("enabled"):
        raise SmokeFailure("Undo after gesture=true drag did not enable Redo")
    screenshot = save_screenshot(recorder, client, artifact_dir)
    return {
        "node_target": node["id"],
        "screenshot": screenshot,
        "synced_revision": synced["rendered_revision"],
        "undo_synced_revision": undo_synced["rendered_revision"],
    }


def persistence_scenario(args: argparse.Namespace, artifact_dir: Path) -> JSON:
    """Create/save/relaunch a project through generic targets when supported."""
    project = artifact_dir / "smoke-project.xmi"
    first = Recorder()
    with McpClient(args.binary, display=args.display, timeout=args.timeout) as client:
        assert_tools(client, first)
        state = snapshot(first, client)
        new_target = target(state, "file.new")
        first.call(client, "ui_select", {"target_id": new_target["id"]})
        state = structured_content(first.call(client, "ui_set_text", {"value": str(project.resolve())}))
        diagram = target(state, "diagram.new.class")
        state = select_and_click(first, client, str(diagram["id"]))
        class_tool = target(state, "tool.class")
        select_and_click(first, client, str(class_tool["id"]))
        state = snapshot(first, client)
        canvas = target(state, "canvas")
        first.call(client, "ui_select", {"target_id": canvas["id"]})
        state = structured_content(first.call(client, "ui_click", {"x": 120.0, "y": 120.0}))
        save = target(state, "file.save")
        select_and_click(first, client, str(save["id"]))
        sync(first, client, snapshot(first, client))
    if not project.is_file():
        raise SmokeFailure(f"save target returned success but did not create {project}")
    second = Recorder()
    with McpClient(args.binary, file_path=project, display=args.display, timeout=args.timeout) as client:
        assert_tools(client, second)
        state = activate_diagram(second, client, snapshot(second, client))
        node = target(state, "node:")
        second.call(client, "ui_select", {"target_id": node["id"]})
        moved = structured_content(
            second.call(client, "ui_drag", {"x": 2000.0, "y": 1500.0, "gesture": True})
        )
        undo = target(moved, "history.undo")
        if not undo.get("enabled"):
            raise SmokeFailure("relaunch gesture=true drag did not enable Undo")
        sync(second, client, moved)
        undone = select_and_click(second, client, str(undo["id"]))
        sync(second, client, undone)
        screenshot = save_screenshot(second, client, artifact_dir)
    return {
        "_initial_steps": first.entries,
        "_relaunch_steps": second.entries,
        "node_target": node["id"],
        "project": str(project),
        "screenshot": screenshot,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/debug/umbrello")
    parser.add_argument("--input-xmi", type=Path, help="read-only model for the readonly scenario")
    parser.add_argument("--display", help="DISPLAY override; otherwise inherit the environment")
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--scenario", choices=("readonly", "persistence"), default="readonly")
    args = parser.parse_args()
    artifact_dir = args.artifact_dir.resolve()
    artifact_dir.mkdir(parents=True, exist_ok=True)
    transcript_path = artifact_dir / "transcript.json"
    try:
        if args.scenario == "readonly":
            if args.input_xmi is None:
                raise SmokeFailure("--input-xmi is required for the read-only scenario")
            recorder = Recorder()
            with McpClient(args.binary, file_path=args.input_xmi, display=args.display, timeout=args.timeout) as client:
                result = readonly_scenario(client, recorder, artifact_dir)
            transcript: JSON = {"result": result, "scenario": args.scenario, "steps": recorder.entries}
        else:
            result = persistence_scenario(args, artifact_dir)
            initial_steps = result.pop("_initial_steps")
            relaunch_steps = result.pop("_relaunch_steps")
            transcript = {
                "result": result,
                "scenario": args.scenario,
                "steps": [
                    {"entries": initial_steps, "phase": "create-save"},
                    {"entries": relaunch_steps, "phase": "relaunch-inspect"},
                ],
            }
        transcript_path.write_text(json.dumps(transcript, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(json.dumps({"artifact_dir": str(artifact_dir), "result": result, "scenario": args.scenario}, sort_keys=True))
        return 0
    except (McpError, SmokeFailure, OSError) as error:
        failure = {"error": str(error), "scenario": args.scenario}
        transcript_path.write_text(json.dumps(failure, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(f"umbrello smoke failure: {error}; transcript={transcript_path}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
