use codex_app_server_protocol::ClientInfo;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::InitializeCapabilities;
use codex_app_server_protocol::InitializeParams;
use codex_app_server_protocol::RemoteClientCapabilities;
use codex_app_server_protocol::RequestId;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn initialize_serializes_remote_client_capabilities() -> anyhow::Result<()> {
    let request = ClientRequest::Initialize {
        request_id: RequestId::Integer(42),
        params: InitializeParams {
            client_info: ClientInfo {
                name: "codex_remote".to_string(),
                title: Some("Codex Remote Client".to_string()),
                version: "0.1.0".to_string(),
            },
            capabilities: Some(InitializeCapabilities {
                experimental_api: true,
                opt_out_notification_methods: None,
                remote_client: Some(RemoteClientCapabilities {
                    renders_diffs: true,
                    answers_approvals: true,
                    supports_file_attachments: false,
                    receives_command_output: true,
                }),
            }),
        },
    };

    assert_eq!(
        json!({
            "method": "initialize",
            "id": 42,
            "params": {
                "clientInfo": {
                    "name": "codex_remote",
                    "title": "Codex Remote Client",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "experimentalApi": true,
                    "optOutNotificationMethods": null,
                    "remoteClient": {
                        "rendersDiffs": true,
                        "answersApprovals": true,
                        "supportsFileAttachments": false,
                        "receivesCommandOutput": true
                    }
                }
            }
        }),
        serde_json::to_value(&request)?,
    );
    Ok(())
}
