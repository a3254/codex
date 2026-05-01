# Remote Access Product Plan

This plan captures the next product evolution for Codex remote access with emphasis on self-hosted VPS operation and WhatsApp-style mobile control. It is intentionally scoped to app-server and client integration surfaces, not `codex-core`.

## Product Reality

Codex is primarily a local coding agent with CLI, IDE, desktop, SDK, and cloud entry points. The local app-server is the control surface for rich clients: it exposes JSON-RPC over stdio by default, experimental websocket and unix socket transports, thread and turn APIs, command execution, file APIs, approvals, MCP, and remote-control status notifications.

Remote access is present but not yet a self-hosted product:

- Direct websocket app-server can listen on `ws://IP:PORT` and supports bearer auth.
- Loopback websocket plus SSH port forwarding is the safest supported remote workflow today.
- Remote control exists behind an under-development feature flag and currently assumes ChatGPT or localhost relay URLs.
- Device-key signing is intentionally local-only and should stay that way.
- No first-party WhatsApp, Telegram, Signal, or SMS gateway exists in this repo.

Evidence sources:

- `codex-rs/app-server/README.md` documents websocket, auth, health probes, API primitives, device-key APIs, and `remoteControl/status/changed`.
- `codex-rs/app-server/src/transport/remote_control/` contains the relay transport, enrollment, protocol, and client tracking pieces.
- `scripts/start-codex-exec.sh` shows repo sync, remote build, remote server start, and SSH tunnel patterns useful for VPS experimentation.
- Market comparables in 2026 emphasize mobile steering, approvals, durable sessions, and background coding rather than full coding on a phone. Examples observed during this skill run include Claude Code Remote Control, Cursor Background Agents web/mobile, GitHub Copilot remote steering, and third-party mobile agent controllers such as Vicoa and Termly. Confidence: medium because this evidence comes from public product positioning, not Codex user analytics.

## Problem Backlog

1. Remote app-server is powerful but hard to recommend for a VPS.
   Evidence: websocket is still marked experimental and non-loopback exposure requires careful auth, TLS, and token handling.
   Impact: high. Confidence: high.

2. Mobile users need to unblock agents, not edit code on a phone.
   Evidence: app-server has approvals, streaming events, interrupts, and turn steering; market comparables center on mobile status, approval, and review loops.
   Impact: high. Confidence: medium.

3. WhatsApp is attractive as a universal mobile inbox but expensive to get wrong.
   Evidence: the repo has no messaging gateway; official WhatsApp Business Platform requires webhook handling, opt-in, templates for outbound business-initiated messages, rate limits, and policy compliance.
   Impact: medium-high. Confidence: medium.

4. Self-hosting changes the trust boundary.
   Evidence: app-server can expose shell, filesystem, MCP tools, and approval flows. A remote client must not turn the user's VPS into an unaudited root-equivalent automation endpoint.
   Impact: high. Confidence: high.

5. There is no product contract for self-hosted relay or channel gateway behavior.
   Evidence: existing remote-control relay is ChatGPT-oriented; arbitrary self-hosted relay support is not a URL-only change.
   Impact: medium. Confidence: high.

## Opportunity Areas

1. Codex Remote Host Mode
   Outcome: a user can run Codex on a home machine or VPS and control it from another device with clear security defaults.
   Success metric: a new user can complete a remote app-server setup without exposing an unauthenticated listener.

2. Mobile Approval Console
   Outcome: a user can see running work, approve or deny risky actions, steer a turn, interrupt a turn, and review final diffs from a mobile-sized client.
   Success metric: fewer turns remain blocked on approvals for more than five minutes.

3. Channel Gateway
   Outcome: chat messages from a mobile channel can start or continue Codex threads without embedding WhatsApp-specific logic into core agent code.
   Success metric: inbound messages map idempotently to threads and outbound replies preserve approval and safety semantics.

4. Secure Self-Hosted Runtime
   Outcome: self-hosted operation has explicit guardrails: auth, TLS, sandbox posture, audit logs, revocation, and tool boundaries.
   Success metric: every remote-capable setup path documents required auth and revocation behavior.

## Candidate Features

| Feature | MVP | V1 | Future |
| --- | --- | --- | --- |
| Direct VPS app-server guide | Document loopback plus SSH tunnel and authenticated websocket | Add hardened reverse-proxy and systemd examples | Add install command that generates token files and health checks |
| Mobile approval console | Protocol contract for required API calls and notifications | PWA client using app-server websocket | Push notifications and offline-safe approval queue |
| Channel gateway | Architecture spec for WhatsApp-style gateway | Reference gateway for one chat channel | Multi-channel gateway with policy-aware templates |
| Self-hosted relay | Identify required relay/auth changes | Experimental config for approved self-hosted relay origins | Durable relay service with pairing and revocation UI |
| Audit and safety posture | Checklist in docs | Structured remote audit events | Admin policy controls for remote clients |

Removed from immediate consideration:

- Unofficial WhatsApp Web automation. It creates account and policy risk and cannot be a first-party recommendation.
- MCP-only WhatsApp inbound control. MCP can expose tools, but it cannot by itself receive external webhooks and initiate user turns.
- Broad `codex-core` changes. The needed primitives already live in app-server, app-server-client, protocol, MCP, and docs.

## Ranked Roadmap

### Now

1. Remote Access Product Contract
   Implemented by this document. Defines supported prototypes, non-goals, security posture, metrics, and execution plan without requiring external credentials.

2. Direct VPS Hardening Docs
   Add app-server README guidance that self-hosted remote access should start with SSH forwarding or authenticated websocket behind TLS.

3. WhatsApp Gateway Contract
   Specify that WhatsApp is an external app-server client gateway, not core agent logic or MCP-only inbound plumbing.

Now is intentionally one measurable wedge: safe direct VPS setup plus a fake channel gateway contract. Production WhatsApp, self-hosted relay, audit protocol changes, and PWA work stay outside this first slice.

### Next

1. Reference Channel Gateway Spike
   Add a small SDK-backed example that maps webhook-like events to `thread/start`, `turn/start`, `turn/steer`, and `turn/interrupt`. Use local fixtures instead of real WhatsApp credentials. Initial executable artifact: `codex_app_server.channel_gateway` plus `sdk/python/examples/15_channel_gateway/`.

2. Remote Client Capability Profile
   Add a protocol or client convention that declares whether a remote client can render diffs, answer approvals, handle file attachments, and receive command output.

3. Remote Audit Log
   Add structured app-server events for remote approvals, denied actions, interrupts, and external gateway identity.

### Later

1. Self-Hosted Remote-Control Relay
   Extend relay URL policy and enrollment only after auth, device binding, revocation, and operational ownership are settled.

2. Official WhatsApp Business Gateway
   Build against the official WhatsApp Business Platform only after product owners can provide credentials, templates, webhook verification, and policy requirements.

3. Mobile PWA
   Build a focused UI for status, approvals, steering, interrupt, summaries, and diffs.

## Named Product Concept

Codex Remote Host

Codex Remote Host lets users keep code, tools, and secrets on a machine they control while steering Codex from lightweight remote clients. The main journey is: install or start app-server on a host, pair a remote client, start or resume a thread, receive progress updates, approve risky actions, steer or interrupt work, and review the final change.

## Architecture Direction

### Direct VPS

Use existing app-server websocket APIs.

- Preferred first path: bind app-server to loopback and connect through SSH port forwarding.
- Explicit remote path: bind app-server to a private interface, require websocket bearer auth, and terminate TLS at a reverse proxy.
- Clients must call `initialize`, then use thread and turn APIs.
- Clients must subscribe to or handle approval, item, command, file, and completion notifications.

### WhatsApp-Style Gateway

Build the gateway as a separate app-server client.

- The gateway owns WhatsApp webhook verification, message deduplication, sender identity, templates, and outbound delivery.
- The gateway maps each WhatsApp conversation to a Codex thread.
- Inbound user messages become `turn/start`, `turn/steer`, or `turn/interrupt`.
- The gateway must bind each sender to an explicit project allowlist and capability profile before it can start or steer turns.
- The gateway must not let channel messages override `cwd`, sandbox policy, approval policy, model provider, filesystem permissions, MCP servers, or network policy unless a separate trusted admin flow grants that capability.
- Approval prompts become constrained reply options such as approve, deny, ask, stop.
- Approval prompts must include the exact action being approved, cwd, target files or network host, permission expansion, and a short risk label. Summaries may shorten logs, but they must not hide the action that will execute.
- Large outputs become summaries with links to a web console or PR, not raw terminal dumps.
- Codex should expose no WhatsApp credentials to the model unless a separately configured MCP tool is intentionally granted.

### Remote-Control Relay

Treat self-hosted remote-control relay as a separate future track. The existing relay protocol has useful reconnect and enrollment concepts, but current URL policy, auth assumptions, and ChatGPT integration mean arbitrary VPS relay support needs a full security design.

## Security Requirements

- Never expose a non-loopback app-server listener without auth.
- Prefer token files or signed bearer tokens over command-line secrets.
- Use TLS for remote websocket traffic outside SSH tunnels.
- Keep device-key APIs local-only.
- Keep WhatsApp credentials in the gateway, not in core Codex.
- Treat phone numbers as weak identity until paired with a trusted account or project membership.
- Verify webhook signatures and reject replayed message ids.
- Avoid sending secrets, raw patches, full logs, or unrestricted file links into chat apps.
- Require link access control for any web console, diff, or artifact URL sent through a channel gateway.
- Require explicit approval for destructive command, file, and network actions in remote sessions.
- Log remote client identity, gateway identity, approval decisions, interrupts, and thread mapping changes.
- Provide a revocation path for remote clients and channel gateways.

## Validation Plan

Proceed when:

- A developer can follow the docs to run app-server through SSH forwarding or authenticated websocket.
- A local fake channel gateway can drive a thread using only app-server APIs.
- Approval, interrupt, and turn completion behavior can be represented in a mobile/chat-sized response.

Revise when:

- The gateway needs app-server behavior that is only available through private internals.
- Approval prompts cannot be safely represented in a constrained chat UI.
- Remote setup requires users to paste secrets into shell history or expose unauthenticated listeners.

Stop or defer when:

- Work requires official WhatsApp credentials, approved templates, paid messaging access, or production webhook infrastructure.
- Work requires changing local-only device-key boundaries.

## Execution Plan

1. Land this product contract and app-server README pointer.
2. Add a fixture-driven channel gateway spike under SDK examples.
3. Add tests around the spike's thread mapping and approval message formatting.
4. Add remote audit event coverage if gateway identity becomes part of app-server protocol.
5. Revisit self-hosted relay only after the direct VPS and gateway paths prove useful.

Definition of done:

- Remote setup guidance is explicit about safe defaults.
- WhatsApp-style inbound control has an app-server-client architecture, not core-agent coupling.
- Every new remote capability has validation, docs, and a rollback path.

## Metrics And Iteration

Track:

- Remote setup success rate: completed setup attempts divided by started setup attempts from docs or installer telemetry. Baseline comes from the first internal pilot; target is 80% completion. Owner: app-server integration lead.
- Approval response latency: median and p90 time from approval notification emitted to approval response received. Baseline comes from current local app-server approval events; target is p90 under five minutes for remote sessions. Owner: client experience lead.
- Long-blocked turns: count and rate of turns waiting more than five minutes on approval, divided by all turns that requested approval. Owner: app-server integration lead.
- Remote session stability: disconnect and reconnect counts per remote session hour, sourced from transport events. Target is fewer than one unexpected disconnect per active session hour in internal pilot. Owner: remote transport lead.
- Approval outcomes: number of approved, denied, expired, and superseded approvals per remote session. Owner: safety/product lead.
- Gateway idempotency: duplicate inbound message ids suppressed divided by duplicate ids observed in gateway logs. Target is 100% suppression in fixture tests. Owner: gateway owner.
- Support signal: weekly count of issues or tickets mentioning remote setup, mobile approval, WhatsApp, or VPS, tagged by root cause. Owner: support/product triage.

Launch review:

- Review metrics one week after a prototype is used internally.
- Compare blocked-turn duration and setup failures against baseline.
- Feed unresolved failures into the next product-improvement cycle.
