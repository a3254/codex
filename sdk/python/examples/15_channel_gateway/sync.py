import sys
from pathlib import Path

_EXAMPLES_ROOT = Path(__file__).resolve().parents[1]
if str(_EXAMPLES_ROOT) not in sys.path:
    sys.path.insert(0, str(_EXAMPLES_ROOT))

from _bootstrap import ensure_local_sdk_src, runtime_config

ensure_local_sdk_src()

from codex_app_server import Codex
from codex_app_server.channel_gateway import (
    ChannelGateway,
    ChannelMessage,
    SenderBinding,
    approval_prompt_text,
)

messages = [
    ChannelMessage(
        message_id="fixture-1",
        sender_id="whatsapp:+15551234567",
        text="Inspect this repo and suggest the smallest useful fix.",
    ),
    ChannelMessage(
        message_id="fixture-2",
        sender_id="whatsapp:+15551234567",
        text="Keep the answer short and do not edit files yet.",
    ),
    ChannelMessage(
        message_id="fixture-2",
        sender_id="whatsapp:+15551234567",
        text="Keep the answer short and do not edit files yet.",
    ),
]

bindings = {
    "whatsapp:+15551234567": SenderBinding(
        sender_id="whatsapp:+15551234567",
        cwd=str(Path.cwd()),
        model="gpt-5.4",
    )
}

with Codex(config=runtime_config()) as codex:
    gateway = ChannelGateway.for_codex(codex, bindings)
    for message in messages:
        result = gateway.handle_message(message)
        print(result.kind, result.thread_id, result.turn_id, result.text)

print(
    approval_prompt_text(
        {
            "type": "command",
            "cwd": str(Path.cwd()),
            "command": "pytest sdk/python/tests/test_channel_gateway.py",
            "risk": "runs a local test command",
        }
    )
)
