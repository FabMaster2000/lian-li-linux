#!/usr/bin/env python3
import asyncio
import json
import sys
from pathlib import Path

try:
    import websockets
except ImportError as exc:
    raise SystemExit(
        "python module 'websockets' is required; install python3-websockets or run 'pip install websockets'"
    ) from exc


async def main() -> int:
    if len(sys.argv) < 5:
        print(
            "usage: await-websocket-event.py <ws-url> <event-type> <output-path> <ready-path> [timeout-seconds]",
            file=sys.stderr,
        )
        return 2

    url = sys.argv[1]
    event_type = sys.argv[2]
    output_path = Path(sys.argv[3])
    ready_path = Path(sys.argv[4])
    timeout_seconds = float(sys.argv[5]) if len(sys.argv) >= 6 else 10.0

    output_path.parent.mkdir(parents=True, exist_ok=True)
    ready_path.parent.mkdir(parents=True, exist_ok=True)

    try:
        async with websockets.connect(url, ping_interval=None) as websocket:
            ready_path.write_text("ready\n", encoding="utf-8")

            while True:
                payload = await asyncio.wait_for(
                    websocket.recv(),
                    timeout=timeout_seconds,
                )
                event = json.loads(payload)
                if event.get("type") != event_type:
                    continue

                output_path.write_text(
                    json.dumps(event, indent=2, sort_keys=True) + "\n",
                    encoding="utf-8",
                )
                return 0
    finally:
        if ready_path.exists():
            ready_path.unlink()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
