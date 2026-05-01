from __future__ import annotations

from dataclasses import dataclass, field
from typing import Protocol

from .api import Codex, TextInput, Thread, TurnHandle
from .generated.v2_all import AskForApproval, SandboxMode


class ChannelGatewayError(ValueError):
    """Raised when an inbound channel message violates gateway policy."""


class ChannelCodex(Protocol):
    def thread_start(
        self,
        *,
        approval_policy: AskForApproval | None = None,
        cwd: str | None = None,
        model: str | None = None,
        sandbox: SandboxMode | None = None,
    ) -> Thread: ...


@dataclass(frozen=True, slots=True)
class SenderBinding:
    sender_id: str
    cwd: str
    model: str | None = None
    approval_policy: AskForApproval | None = "on-request"
    sandbox: SandboxMode | None = "workspace-write"


@dataclass(frozen=True, slots=True)
class ChannelMessage:
    message_id: str
    sender_id: str
    text: str


@dataclass(frozen=True, slots=True)
class GatewayResult:
    kind: str
    thread_id: str | None = None
    turn_id: str | None = None
    text: str | None = None


@dataclass(slots=True)
class _ConversationState:
    thread: Thread
    active_turn: TurnHandle | None = None


@dataclass(slots=True)
class ChannelGateway:
    """Map mobile/chat messages onto a constrained app-server client.

    The gateway deliberately owns sender authorization and project binding. Inbound
    channel text can start, steer, or interrupt turns, but cannot override cwd,
    sandbox, approval policy, model, filesystem permissions, MCP servers, or other
    privileged app-server settings.
    """

    codex: ChannelCodex
    bindings: dict[str, SenderBinding]
    _conversations: dict[str, _ConversationState] = field(default_factory=dict)
    _processed_message_ids: set[str] = field(default_factory=set)

    @classmethod
    def for_codex(
        cls, codex: Codex, bindings: dict[str, SenderBinding]
    ) -> "ChannelGateway":
        return cls(codex=codex, bindings=bindings)

    def handle_message(self, message: ChannelMessage) -> GatewayResult:
        if message.message_id in self._processed_message_ids:
            return GatewayResult(kind="duplicate", text="Message already processed.")

        binding = self.bindings.get(message.sender_id)
        if binding is None:
            raise ChannelGatewayError(f"sender {message.sender_id!r} is not allowed")

        text = message.text.strip()
        if not text:
            raise ChannelGatewayError("message text is empty")

        state = self._conversations.get(message.sender_id)
        self._processed_message_ids.add(message.message_id)

        if text.lower() in {"/stop", "stop"}:
            if state is None or state.active_turn is None:
                return GatewayResult(kind="idle", text="No active turn to stop.")
            state.active_turn.interrupt()
            interrupted_turn_id = state.active_turn.id
            state.active_turn = None
            return GatewayResult(
                kind="interrupted",
                thread_id=state.thread.id,
                turn_id=interrupted_turn_id,
                text="Turn interrupted.",
            )

        if state is None:
            thread = self.codex.thread_start(
                approval_policy=binding.approval_policy,
                cwd=binding.cwd,
                model=binding.model,
                sandbox=binding.sandbox,
            )
            state = _ConversationState(thread=thread)
            self._conversations[message.sender_id] = state
            turn = thread.turn(TextInput(text))
            state.active_turn = turn
            return GatewayResult(
                kind="started",
                thread_id=thread.id,
                turn_id=turn.id,
                text="Started Codex turn.",
            )

        if state.active_turn is None:
            turn = state.thread.turn(TextInput(text))
            state.active_turn = turn
            return GatewayResult(
                kind="started",
                thread_id=state.thread.id,
                turn_id=turn.id,
                text="Started Codex turn.",
            )

        state.active_turn.steer(TextInput(text))
        return GatewayResult(
            kind="steered",
            thread_id=state.thread.id,
            turn_id=state.active_turn.id,
            text="Steered active Codex turn.",
        )

    def mark_turn_complete(self, sender_id: str, turn_id: str) -> None:
        state = self._conversations.get(sender_id)
        if state is not None and state.active_turn is not None:
            if state.active_turn.id == turn_id:
                state.active_turn = None


def approval_prompt_text(action: dict[str, object]) -> str:
    """Format a compact approval prompt for a constrained mobile channel."""

    action_type = str(action.get("type", "action"))
    cwd = str(action.get("cwd", "unknown cwd"))
    command = action.get("command")
    files = action.get("files")
    host = action.get("host")
    risk = str(action.get("risk", "review required"))

    if command is not None:
        target = f"command: {command}"
    elif files is not None:
        target = f"files: {files}"
    elif host is not None:
        target = f"network host: {host}"
    else:
        target = "target: unavailable"

    return (
        f"Approval required\n"
        f"type: {action_type}\n"
        f"cwd: {cwd}\n"
        f"{target}\n"
        f"risk: {risk}\n"
        "Reply approve, deny, ask, or stop."
    )
