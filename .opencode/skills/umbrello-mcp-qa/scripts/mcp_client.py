#!/usr/bin/env python3
"""Standard-library MCP stdio client for the native Umbrello QA server."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import threading
import time
from typing import Any


JSON = dict[str, Any]
EXPECTED_TOOLS = {
    "ui_inspect",
    "ui_select",
    "ui_click",
    "ui_set_text",
    "ui_drag",
    "ui_sync",
    "ui_screenshot",
}


class McpError(RuntimeError):
    """Raised when the subprocess, JSON-RPC protocol, or tool call fails."""


class McpClient:
    """Launch and communicate with Umbrello's newline-delimited MCP server."""

    def __init__(
        self,
        binary: str | Path,
        *,
        file_path: str | Path | None = None,
        display: str | None = None,
        timeout: float = 15.0,
    ) -> None:
        self.binary = str(binary)
        self.file_path = str(file_path) if file_path else None
        self.display = display
        self.timeout = timeout
        self.process: subprocess.Popen[str] | None = None
        self._next_id = 1
        self.stderr_lines: list[str] = []
        self._stderr_thread: threading.Thread | None = None

    def __enter__(self) -> "McpClient":
        self.start()
        return self

    def __exit__(self, _type: object, _value: object, _traceback: object) -> None:
        self.close()

    def start(self) -> None:
        """Start Umbrello and complete MCP initialization."""
        if self.process is not None:
            raise McpError("MCP client is already started")
        environment = os.environ.copy()
        if self.display:
            environment["DISPLAY"] = self.display
        command = [self.binary, "--mcp-stdio"]
        if self.file_path:
            command.append(self.file_path)
        try:
            self.process = subprocess.Popen(
                command,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,
                env=environment,
            )
        except OSError as error:
            raise McpError(f"cannot launch {' '.join(command)!r}: {error}") from error
        self._stderr_thread = threading.Thread(target=self._drain_stderr, daemon=True)
        self._stderr_thread.start()
        try:
            self.request(
                "initialize",
                {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {"name": "umbrello-mcp-qa", "version": "1"},
                },
            )
            self.notify("notifications/initialized", {})
        except BaseException:
            # __enter__ does not return when startup fails, so it cannot
            # invoke __exit__. Ensure a timed-out/broken initialization never
            # leaves a native eframe process behind.
            self.close()
            raise

    def _drain_stderr(self) -> None:
        assert self.process is not None and self.process.stderr is not None
        for line in self.process.stderr:
            self.stderr_lines.append(line.rstrip())

    def _require_process(self) -> subprocess.Popen[str]:
        if self.process is None or self.process.stdin is None or self.process.stdout is None:
            raise McpError("MCP client is not started")
        return self.process

    def _send(self, message: JSON) -> None:
        process = self._require_process()
        if process.poll() is not None:
            raise McpError(self._exit_message("Umbrello exited before request"))
        assert process.stdin is not None
        process.stdin.write(json.dumps(message, sort_keys=True, separators=(",", ":")) + "\n")
        process.stdin.flush()

    def request(self, method: str, params: JSON) -> JSON:
        """Send a JSON-RPC request and return its correlated response."""
        request_id = self._next_id
        self._next_id += 1
        self._send({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params})
        deadline = time.monotonic() + self.timeout
        process = self._require_process()
        assert process.stdout is not None
        while time.monotonic() < deadline:
            remaining = max(0.0, deadline - time.monotonic())
            ready, _, _ = select.select([process.stdout], [], [], remaining)
            if not ready:
                break
            line = process.stdout.readline()
            if not line:
                raise McpError(self._exit_message("Umbrello closed MCP stdout"))
            try:
                response = json.loads(line)
            except json.JSONDecodeError as error:
                raise McpError(f"invalid JSON-RPC stdout: {line!r}") from error
            if response.get("id") != request_id:
                continue
            if "error" in response:
                raise McpError(f"{method} failed: {response['error']}")
            return response
        raise McpError(self._exit_message(f"timeout after {self.timeout:g}s waiting for {method}"))

    def notify(self, method: str, params: JSON) -> None:
        """Send a JSON-RPC notification without waiting for a response."""
        self._send({"jsonrpc": "2.0", "method": method, "params": params})

    def list_tools(self) -> list[JSON]:
        """Return server-advertised tool definitions and validate their shape."""
        result = self.request("tools/list", {}).get("result", {})
        tools = result.get("tools")
        if not isinstance(tools, list):
            raise McpError(f"tools/list returned no tool list: {result!r}")
        return tools

    def call_tool(self, name: str, arguments: JSON | None = None) -> JSON:
        """Call one generic Umbrello MCP tool."""
        return self.request("tools/call", {"name": name, "arguments": arguments or {}})

    @staticmethod
    def transcript_response(name: str, response: JSON) -> JSON:
        """Return a transcript-safe response copy without screenshot base64."""
        if name != "ui_screenshot":
            return response
        result = response.get("result", {})
        content = result.get("content", [])
        if not isinstance(content, list):
            return response
        safe_content: list[JSON] = []
        for item in content:
            if not isinstance(item, dict) or item.get("type") != "image":
                safe_content.append(item)
                continue
            safe_image: JSON = {key: value for key, value in item.items() if key != "data"}
            try:
                png, _ = decode_image_content(response)
                safe_image.update(
                    {
                        "decoded_byte_length": len(png),
                        "sha256": hashlib.sha256(png).hexdigest(),
                    }
                )
            except McpError as error:
                safe_image["decode_error"] = str(error)
            safe_content.append(safe_image)
        safe_result = dict(result)
        safe_result["content"] = safe_content
        safe_response = dict(response)
        safe_response["result"] = safe_result
        return safe_response

    def close(self) -> None:
        """Close MCP stdin, then terminate and finally kill a lingering child."""
        process = self.process
        if process is None:
            return
        try:
            if process.stdin and not process.stdin.closed:
                process.stdin.close()
            process.wait(timeout=min(3.0, self.timeout))
        except (subprocess.TimeoutExpired, OSError):
            process.terminate()
            try:
                process.wait(timeout=3.0)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3.0)
        finally:
            self.process = None

    def _exit_message(self, prefix: str) -> str:
        process = self.process
        code = process.poll() if process else None
        stderr = "\n".join(self.stderr_lines[-10:])
        return f"{prefix}; exit={code}; stderr={stderr or '<none>'}"


def structured_content(response: JSON) -> JSON:
    """Extract an MCP tool snapshot, accepting object or JSON-string forms."""
    result = response.get("result", {})
    value = result.get("structuredContent")
    if value is None:
        content = result.get("content", [])
        if isinstance(content, list):
            for item in content:
                if isinstance(item, dict) and item.get("type") == "text":
                    value = item.get("text")
                    break
    if isinstance(value, str):
        try:
            value = json.loads(value)
        except json.JSONDecodeError as error:
            raise McpError(f"tool response contains invalid structured JSON: {value!r}") from error
    if not isinstance(value, dict):
        raise McpError(f"tool response has no structured snapshot: {result!r}")
    return value


def decode_image_content(response: JSON) -> tuple[bytes, JSON]:
    """Decode the first MCP image block and optional JSON text metadata safely."""
    content = response.get("result", {}).get("content", [])
    if not isinstance(content, list):
        raise McpError("screenshot response has invalid content")
    image = next((item for item in content if isinstance(item, dict) and item.get("type") == "image"), None)
    if not image or not isinstance(image.get("data"), str):
        raise McpError(f"screenshot response has no image block: {content!r}")
    try:
        png = base64.b64decode(image["data"], validate=True)
    except (ValueError, TypeError) as error:
        raise McpError("screenshot image data is not valid base64") from error
    if not png.startswith(b"\x89PNG\r\n\x1a\n"):
        raise McpError("screenshot image is not a PNG")
    metadata: JSON = {}
    text = next((item.get("text") for item in content if isinstance(item, dict) and item.get("type") == "text"), None)
    if isinstance(text, str):
        try:
            decoded = json.loads(text)
            if isinstance(decoded, dict):
                metadata = decoded
        except json.JSONDecodeError:
            metadata = {"text": text}
    return png, metadata


def _cli() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="target/debug/umbrello")
    parser.add_argument("--file", dest="file_path")
    parser.add_argument("--display")
    parser.add_argument("--timeout", type=float, default=15.0)
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("tools")
    subparsers.add_parser("inspect")
    call = subparsers.add_parser("call")
    call.add_argument("name")
    call.add_argument("arguments", nargs="?", default="{}", help="JSON object")
    args = parser.parse_args()
    try:
        with McpClient(args.binary, file_path=args.file_path, display=args.display, timeout=args.timeout) as client:
            if args.command == "tools":
                result: Any = client.list_tools()
            elif args.command == "inspect":
                result = structured_content(client.call_tool("ui_inspect"))
            else:
                parsed = json.loads(args.arguments)
                if not isinstance(parsed, dict):
                    raise McpError("call arguments must be a JSON object")
                result = client.call_tool(args.name, parsed)
            print(json.dumps(result, indent=2, sort_keys=True))
            return 0
    except (McpError, json.JSONDecodeError) as error:
        print(f"umbrello MCP client error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(_cli())
