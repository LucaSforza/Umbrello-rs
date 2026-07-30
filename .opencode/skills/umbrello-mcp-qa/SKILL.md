---
name: umbrello-mcp-qa
description: Use when Umbrello MCP, automated GUI QA, ui_inspect/ui_select/ui_click/ui_set_text/ui_drag/ui_sync/ui_screenshot, persistence/reload QA, or reviewer final gates require running the native application.
---

# Umbrello MCP QA

This skill runs reproducible, native Umbrello GUI QA through its seven generic MCP tools. It is **mandatory** for final reviews of GUI automation, MCP, screenshots, persistence UI flows, or mouse-equivalent interactions, and for any request for automated Umbrello GUI QA. Static/unit tests are supporting evidence, not a substitute.

## Prerequisites

- Build the application first: `cargo build -p umbrello` (the default binary is `target/debug/umbrello`).
- A native display is required. Preserve an existing `DISPLAY`; Xwayland commonly supplies `:0`. Wayland sessions normally expose Xwayland for eframe. If no display can launch the native window, report the exact failure and strongest static evidence.
- Umbrello must run with `--mcp-stdio`. Its stdout is protocol-only; diagnostics are captured from stderr by the client and never mixed into JSON-RPC.
- Python 3 standard library is sufficient. Do not use disposable heredocs: use the versioned scripts below.

## Resources and commands

The reusable JSON-RPC client is `scripts/mcp_client.py`:

```sh
python3 .opencode/skills/umbrello-mcp-qa/scripts/mcp_client.py \
  --binary target/debug/umbrello --display "$DISPLAY" tools
python3 .opencode/skills/umbrello-mcp-qa/scripts/mcp_client.py \
  --binary target/debug/umbrello --file ../test/test-COG.xmi inspect
```

The scenario runner is `scripts/umbrello_smoke.py`:

```sh
# Read-only native gesture/sync/undo/screenshot flow.
python3 .opencode/skills/umbrello-mcp-qa/scripts/umbrello_smoke.py \
  --scenario readonly --binary target/debug/umbrello \
  --input-xmi ../test/test-COG.xmi \
  --artifact-dir /tmp/opencode/umbrello-mcp-qa-validation

# Writable project/create/save/relaunch flow, only when the target surface supports it.
python3 .opencode/skills/umbrello-mcp-qa/scripts/umbrello_smoke.py \
  --scenario persistence --binary target/debug/umbrello \
  --artifact-dir /tmp/opencode/umbrello-mcp-qa-persistence
```

Each smoke run writes deterministic `transcript.json` and a decoded `screenshot.png` to the selected artifact directory. Screenshot base64 is redacted from transcripts; they retain image MIME information, decoded byte length, SHA-256, and metadata. The persistence project is placed in that explicit artifact directory. Both scripts enforce request timeouts, clean up failed initialization, close stdin, then terminate/kill child processes if necessary.

## Required QA workflow

1. Initialize MCP, issue `tools/list`, and verify exactly these seven tool names: `ui_inspect`, `ui_select`, `ui_click`, `ui_set_text`, `ui_drag`, `ui_sync`, and `ui_screenshot`.
2. Inspect and dynamically discover targets. Never hard-code UUID-derived `diagram:` or `node:` IDs.
3. Select and activate a diagram/visible target, mutate visible state through generic tools, then synchronize with the returned `state_revision` using `ui_sync`.
4. For node movement, select a discovered node and invoke `ui_drag` with `gesture: true`; verify history changes, invoke Undo, and synchronize again.
5. After Undo, synchronize the undo revision before requesting `ui_screenshot`; decode the MCP image content to PNG and record metadata plus a redacted image digest in the transcript.
6. For persistence QA, create a project at an explicit temporary/artifact path, create a Class diagram and Class, save, relaunch with the saved XMI, inspect that diagram/node targets survive, then repeat `gesture: true`, sync, Undo, sync, and screenshot when the generic target surface permits those actions.

`state_revision` is not proof of a rendered frame: pass it to `ui_sync` and require `rendered_revision >= state_revision`. MCP image content is base64 PNG; use the client decoder rather than trusting text metadata alone.

## Blockers and troubleshooting

- Use `--display :0` or allow the script to inherit `DISPLAY`. Report native-launch, JSON-RPC timeout, tool error, target-unavailable, or screenshot/decode errors verbatim.
- If no active diagram or nodes appear, inspect targets, activate a discovered `diagram:` target with `ui_select` then `ui_click`, and inspect again.
- If a persistence action is absent/disabled, record the target ID, enabled state, response, and use the strongest supported combination of read-only MCP QA and process relaunch. Do not claim that static tests replace the live workflow.
- Preserve generated artifacts for the review report; do not write into C++ reference directories.
