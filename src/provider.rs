use std::io::{BufRead, BufReader, Read as _};
use std::process::{Command, Stdio};

use serde::Deserialize;

/// Events emitted by the Claude provider during a turn.
pub enum ProviderEvent {
    TextDelta(String),
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        tool_use_id: String,
        output: String,
        is_error: bool,
    },
    Complete,
    Error(String),
}

/// Raw stream-json event from `claude -p --output-format stream-json`.
#[derive(Deserialize)]
struct RawEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    message: Option<RawMessage>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct RawMessage {
    #[serde(default)]
    content: Vec<RawContentBlock>,
}

#[derive(Deserialize)]
struct RawContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<serde_json::Value>,
    #[serde(default)]
    content: Option<serde_json::Value>,
    #[serde(default)]
    tool_use_id: Option<String>,
    #[serde(default)]
    is_error: Option<bool>,
}

/// Check if the `claude` CLI is available on PATH.
pub fn is_claude_available() -> bool {
    Command::new("claude")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Run a claude turn synchronously (blocking). Call from a background thread.
pub fn run_turn(
    prompt: &str,
    working_dir: Option<&str>,
    tx: &std::sync::mpsc::Sender<ProviderEvent>,
) {
    let mut cmd = Command::new("claude");
    cmd.args(["-p", prompt, "--output-format", "stream-json"]);

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            tx.send(ProviderEvent::Error(format!(
                "Failed to spawn `claude`: {e}"
            )))
            .ok();
            tx.send(ProviderEvent::Complete).ok();
            return;
        }
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            tx.send(ProviderEvent::Error(
                "Failed to capture stdout".to_string(),
            ))
            .ok();
            tx.send(ProviderEvent::Complete).ok();
            return;
        }
    };

    let reader = BufReader::new(stdout);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                tx.send(ProviderEvent::Error(format!("Read error: {e}")))
                    .ok();
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        if let Ok(event) = serde_json::from_str::<RawEvent>(&line) {
            for pe in parse_event(event) {
                tx.send(pe).ok();
            }
        }
    }

    let status = child.wait();
    if let Ok(status) = &status {
        if !status.success() {
            if let Some(mut stderr) = child.stderr.take() {
                let mut buf = String::new();
                stderr.read_to_string(&mut buf).ok();
                if !buf.is_empty() {
                    tx.send(ProviderEvent::Error(buf)).ok();
                }
            }
        }
    }

    tx.send(ProviderEvent::Complete).ok();
}

fn summarize_tool_input(name: &str, input: &serde_json::Value) -> String {
    match name {
        "Bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| truncate(s, 80))
            .unwrap_or_default(),
        "Read" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Write" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Edit" => input
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Glob" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "Grep" => input
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => serde_json::to_string(input)
            .map(|s| truncate(&s, 60))
            .unwrap_or_default(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

fn extract_tool_result_text(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    parts.push(text.to_string());
                }
            }
            parts.join("\n")
        }
        _ => serde_json::to_string_pretty(content).unwrap_or_default(),
    }
}

fn parse_event(event: RawEvent) -> Vec<ProviderEvent> {
    let mut events = Vec::new();

    match event.event_type.as_str() {
        "assistant" | "user" => {
            if let Some(msg) = event.message {
                for block in msg.content {
                    match block.block_type.as_str() {
                        "text" => {
                            if let Some(text) = block.text {
                                events.push(ProviderEvent::TextDelta(text));
                            }
                        }
                        "tool_use" => {
                            let id = block
                                .id
                                .unwrap_or_else(|| format!("tool_{}", events.len()));
                            let name =
                                block.name.unwrap_or_else(|| "unknown".to_string());
                            let input = block
                                .input
                                .as_ref()
                                .map(|v| summarize_tool_input(&name, v))
                                .unwrap_or_default();
                            events.push(ProviderEvent::ToolUse { id, name, input });
                        }
                        "tool_result" => {
                            let tool_use_id =
                                block.tool_use_id.unwrap_or_default();
                            let output = block
                                .content
                                .as_ref()
                                .map(extract_tool_result_text)
                                .unwrap_or_default();
                            let is_error = block.is_error.unwrap_or(false);
                            events.push(ProviderEvent::ToolResult {
                                tool_use_id,
                                output,
                                is_error,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
        "content_block_delta" => {
            if let Some(text) = event.content {
                events.push(ProviderEvent::TextDelta(text));
            }
        }
        "result" => {
            if let Some(err) = event.error {
                events.push(ProviderEvent::Error(err));
            }
        }
        "error" => {
            let msg = event
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            events.push(ProviderEvent::Error(msg));
        }
        _ => {}
    }

    events
}
