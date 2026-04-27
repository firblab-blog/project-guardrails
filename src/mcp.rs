use std::{
    io::{self, BufRead, Write},
    path::Path,
};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::{
    operations::{
        brief::build_brief,
        check::run_check,
        doctor::run_doctor,
        handoff::{HandoffCreateInput, create_handoff, list_handoffs},
        pre_work::record_pre_work,
        refresh::refresh,
        resume::build_resume,
        status::{build_llm_status, build_status},
        tasks::{TaskListOptions, claim_task, get_task, list_tasks},
        timeline::build_timeline,
    },
    state::tasks::TaskStatus,
};

const PROTOCOL_VERSION: &str = "2025-06-18";

pub fn serve_stdio(target: &Path) -> Result<()> {
    serve(target, io::stdin().lock(), io::stdout().lock())
}

fn serve<R, W>(target: &Path, reader: R, mut writer: W) -> Result<()>
where
    R: BufRead,
    W: Write,
{
    for line in reader.lines() {
        let line = line.context("failed to read MCP stdin")?;
        if line.trim().is_empty() {
            continue;
        }

        let response = match parse_message(&line) {
            Ok(message) => handle_message(target, message),
            Err(response) => Some(response),
        };

        if let Some(response) = response {
            writeln!(writer, "{}", serde_json::to_string(&response)?)?;
            writer.flush()?;
        }
    }

    Ok(())
}

fn parse_message(line: &str) -> std::result::Result<Value, Value> {
    match serde_json::from_str(line) {
        Ok(message) => Ok(message),
        Err(error) => Err(error_response(
            Value::Null,
            -32700,
            format!("failed to parse JSON-RPC message: {error}"),
        )),
    }
}

fn handle_message(target: &Path, message: Value) -> Option<Value> {
    if message.get("error").is_some() || message.get("result").is_some() {
        return None;
    }

    let id = message.get("id").cloned();
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return id.map(|id| error_response(id, -32600, "invalid JSON-RPC request"));
    };

    let id = id?;
    let result = match method {
        "initialize" => Ok(initialize_result()),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool(target, message.get("params").unwrap_or(&Value::Null)),
        "shutdown" => Ok(Value::Null),
        _ => {
            return Some(error_response(
                id,
                -32601,
                format!("method not found: {method}"),
            ));
        }
    };

    Some(match result {
        Ok(result) => success_response(id, result),
        Err(error) => error_response(id, -32602, error.to_string()),
    })
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": "project-guardrails",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "Local-first access to project-guardrails operations for the configured repo. Mutating tools write only repo-local guardrails state or managed blocks."
    })
}

fn call_tool(target: &Path, params: &Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .context("tools/call params.name must be a string")?;
    let arguments = params.get("arguments").unwrap_or(&Value::Null);

    if !tool_definitions()
        .iter()
        .any(|tool| tool.get("name").and_then(Value::as_str) == Some(name))
    {
        bail!("unknown tool: {name}");
    }

    let result = match run_tool(target, name, arguments) {
        Ok(output) => tool_result(output, false),
        Err(error) => tool_result(
            json!({
                "error": {
                    "message": error.to_string()
                }
            }),
            true,
        ),
    };

    Ok(result)
}

fn run_tool(target: &Path, name: &str, arguments: &Value) -> Result<Value> {
    match name {
        "guardrails.pre_work" => to_value(record_pre_work(target)?),
        "guardrails.brief" => to_value(build_brief(target)?),
        "guardrails.resume" => to_value(build_resume(target)?),
        "guardrails.status" => {
            if optional_bool(arguments, "for_llm")?.unwrap_or(true) {
                to_value(build_llm_status(target)?)
            } else {
                to_value(build_status(target)?)
            }
        }
        "guardrails.tasks.list" => to_value(list_tasks(
            target,
            TaskListOptions {
                status: optional_string(arguments, "status")?
                    .map(|status| parse_task_status(&status))
                    .transpose()?,
                owner: optional_string(arguments, "owner")?,
            },
        )?),
        "guardrails.tasks.get" => to_value(get_task(target, required_u32(arguments, "id")?)?),
        "guardrails.tasks.claim" => to_value(claim_task(
            target,
            required_u32(arguments, "id")?,
            required_string(arguments, "owner")?,
        )?),
        "guardrails.handoff.list" => to_value(list_handoffs(target)?),
        "guardrails.handoff.new" => to_value(create_handoff(
            target,
            HandoffCreateInput {
                slug: required_string(arguments, "slug")?,
                title: optional_string(arguments, "title")?,
                task_ids: optional_u32_array(arguments, "task_ids")?.unwrap_or_default(),
                from_git: optional_bool(arguments, "from_git")?.unwrap_or(false),
            },
        )?),
        "guardrails.refresh" => to_value(refresh(
            target,
            optional_bool(arguments, "check")?.unwrap_or(false),
        )?),
        "guardrails.timeline" => to_value(build_timeline(target)?),
        "guardrails.check" => to_value(run_check(target)?),
        "guardrails.doctor" => to_value(run_doctor(target)?),
        _ => unreachable!("unknown tools are checked before dispatch"),
    }
}

fn tool_result(output: Value, is_error: bool) -> Value {
    let text = serde_json::to_string_pretty(&output).expect("JSON values serialize");
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "structuredContent": output,
        "isError": is_error
    })
}

fn to_value<T: Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value).context("failed to serialize operation output")
}

fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn error_response(id: Value, code: i32, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into()
        }
    })
}

fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "guardrails.pre_work",
            "Guardrails Pre-work",
            "MUTATING: record a pre-work run under .guardrails/state/runs/ and return the LLM repo summary.",
            object_schema(Map::new(), Vec::new()),
            false,
            false,
            false,
        ),
        tool(
            "guardrails.brief",
            "Guardrails Brief",
            "Read the current repo briefing without writing guardrails state.",
            object_schema(Map::new(), Vec::new()),
            true,
            false,
            true,
        ),
        tool(
            "guardrails.resume",
            "Guardrails Resume",
            "Read continuation context centered on the latest durable handoff.",
            object_schema(Map::new(), Vec::new()),
            true,
            false,
            true,
        ),
        tool(
            "guardrails.status",
            "Guardrails Status",
            "Read repo status. Defaults to the LLM-oriented status shape unless for_llm is false.",
            object_schema(
                map([(
                    "for_llm",
                    json!({
                        "type": "boolean",
                        "description": "When true or omitted, return the LLM-oriented status summary."
                    }),
                )]),
                Vec::new(),
            ),
            true,
            false,
            true,
        ),
        tool(
            "guardrails.tasks.list",
            "List Guardrails Tasks",
            "Read task records from .guardrails/state/tasks/.",
            object_schema(
                map([
                    (
                        "status",
                        json!({
                            "type": "string",
                            "enum": ["proposed", "approved", "in_progress", "blocked", "done", "dropped"]
                        }),
                    ),
                    (
                        "owner",
                        json!({
                            "type": "string"
                        }),
                    ),
                ]),
                Vec::new(),
            ),
            true,
            false,
            true,
        ),
        tool(
            "guardrails.tasks.get",
            "Get Guardrails Task",
            "Read one task record by numeric id.",
            object_schema(
                map([(
                    "id",
                    json!({
                        "description": "Task id, such as 15 or \"0015\".",
                        "oneOf": [{"type": "integer"}, {"type": "string"}]
                    }),
                )]),
                vec!["id"],
            ),
            true,
            false,
            true,
        ),
        tool(
            "guardrails.tasks.claim",
            "Claim Guardrails Task",
            "MUTATING: set a task owner, move the task to in_progress, and return the changed task record.",
            object_schema(
                map([
                    (
                        "id",
                        json!({
                            "description": "Task id, such as 15 or \"0015\".",
                            "oneOf": [{"type": "integer"}, {"type": "string"}]
                        }),
                    ),
                    (
                        "owner",
                        json!({
                            "type": "string"
                        }),
                    ),
                ]),
                vec!["id", "owner"],
            ),
            false,
            false,
            false,
        ),
        tool(
            "guardrails.handoff.list",
            "List Guardrails Handoffs",
            "Read handoff records from .guardrails/state/handoffs/.",
            object_schema(Map::new(), Vec::new()),
            true,
            false,
            true,
        ),
        tool(
            "guardrails.handoff.new",
            "Create Guardrails Handoff",
            "MUTATING: create a new handoff record under .guardrails/state/handoffs/ and return the changed record.",
            object_schema(
                map([
                    (
                        "slug",
                        json!({
                            "type": "string",
                            "description": "Kebab-case handoff slug."
                        }),
                    ),
                    (
                        "title",
                        json!({
                            "type": "string"
                        }),
                    ),
                    (
                        "task_ids",
                        json!({
                            "type": "array",
                            "items": {
                                "oneOf": [{"type": "integer"}, {"type": "string"}]
                            }
                        }),
                    ),
                    (
                        "from_git",
                        json!({
                            "type": "boolean",
                            "description": "Draft the handoff body from observable local Git state."
                        }),
                    ),
                ]),
                vec!["slug"],
            ),
            false,
            false,
            false,
        ),
        tool(
            "guardrails.refresh",
            "Refresh Guardrails Managed Blocks",
            "MUTATING unless check is true: refresh profile-declared managed blocks and return changed paths.",
            object_schema(
                map([(
                    "check",
                    json!({
                        "type": "boolean",
                        "description": "When true, report stale managed blocks without writing files."
                    }),
                )]),
                Vec::new(),
            ),
            false,
            false,
            true,
        ),
        tool(
            "guardrails.timeline",
            "Guardrails Timeline",
            "Read a newest-first timeline from existing repo-local guardrails state.",
            object_schema(Map::new(), Vec::new()),
            true,
            false,
            true,
        ),
        tool(
            "guardrails.check",
            "Guardrails Check",
            "Run configured local guardrails checks and return structured diagnostics.",
            object_schema(Map::new(), Vec::new()),
            true,
            false,
            false,
        ),
        tool(
            "guardrails.doctor",
            "Guardrails Doctor",
            "Read local guardrails preflight diagnostics and status entries.",
            object_schema(Map::new(), Vec::new()),
            true,
            false,
            true,
        ),
    ]
}

fn tool(
    name: &str,
    title: &str,
    description: &str,
    input_schema: Value,
    read_only: bool,
    destructive: bool,
    idempotent: bool,
) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": input_schema,
        "outputSchema": {
            "type": "object",
            "additionalProperties": true
        },
        "annotations": {
            "readOnlyHint": read_only,
            "destructiveHint": destructive,
            "idempotentHint": idempotent,
            "openWorldHint": false
        }
    })
}

fn object_schema(properties: Map<String, Value>, required: Vec<&str>) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn map<const N: usize>(items: [(&str, Value); N]) -> Map<String, Value> {
    items
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

fn required_string(arguments: &Value, key: &str) -> Result<String> {
    optional_string(arguments, key)?.with_context(|| format!("missing required argument `{key}`"))
}

fn optional_string(arguments: &Value, key: &str) -> Result<Option<String>> {
    let Some(value) = arguments.get(key) else {
        return Ok(None);
    };

    value
        .as_str()
        .map(|value| Some(value.to_string()))
        .with_context(|| format!("argument `{key}` must be a string"))
}

fn optional_bool(arguments: &Value, key: &str) -> Result<Option<bool>> {
    let Some(value) = arguments.get(key) else {
        return Ok(None);
    };

    value
        .as_bool()
        .map(Some)
        .with_context(|| format!("argument `{key}` must be a boolean"))
}

fn required_u32(arguments: &Value, key: &str) -> Result<u32> {
    let value = arguments
        .get(key)
        .with_context(|| format!("missing required argument `{key}`"))?;
    parse_u32(value).with_context(|| format!("argument `{key}` must be a positive integer"))
}

fn optional_u32_array(arguments: &Value, key: &str) -> Result<Option<Vec<u32>>> {
    let Some(value) = arguments.get(key) else {
        return Ok(None);
    };
    let items = value
        .as_array()
        .with_context(|| format!("argument `{key}` must be an array"))?
        .iter()
        .map(parse_u32)
        .collect::<Result<Vec<_>>>()
        .with_context(|| format!("argument `{key}` must contain positive integer ids"))?;

    Ok(Some(items))
}

fn parse_u32(value: &Value) -> Result<u32> {
    if let Some(value) = value.as_u64() {
        return u32::try_from(value).context("integer is too large for a task id");
    }

    value
        .as_str()
        .context("expected integer or string")?
        .parse::<u32>()
        .context("failed to parse integer string")
}

fn parse_task_status(value: &str) -> Result<TaskStatus> {
    match value {
        "proposed" => Ok(TaskStatus::Proposed),
        "approved" => Ok(TaskStatus::Approved),
        "in_progress" => Ok(TaskStatus::InProgress),
        "blocked" => Ok(TaskStatus::Blocked),
        "done" => Ok(TaskStatus::Done),
        "dropped" => Ok(TaskStatus::Dropped),
        _ => bail!("unknown task status `{value}`"),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor, path::Path};

    use serde_json::{Value, json};
    use tempfile::TempDir;

    use super::*;
    use crate::{
        cli::{CiProvider, InitArgs},
        commands::init,
        operations::tasks::{TaskCreateInput, create_task},
        state::tasks::TaskPriority,
    };

    #[test]
    fn serve_in_memory_covers_mcp_protocol_and_tool_dispatch() {
        let temp = TempDir::new().expect("temp dir");
        let repo = temp.path().join("mcp-unit");
        setup_repo(&repo);

        let input = [
            "",
            "{not json}",
            r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"unknown","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":4,"params":{}}"#,
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"guardrails.brief","arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"guardrails.resume","arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"guardrails.status","arguments":{"for_llm":false}}}"#,
            r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"guardrails.tasks.list","arguments":{"status":"in_progress","owner":"MCP Unit"}}}"#,
            r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"guardrails.tasks.get","arguments":{"id":"0001"}}}"#,
            r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"guardrails.tasks.claim","arguments":{"id":1,"owner":"MCP Claimed"}}}"#,
            r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"guardrails.handoff.list","arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"guardrails.handoff.new","arguments":{"slug":"mcp-unit-handoff","title":"MCP Unit Handoff","task_ids":["0001"],"from_git":false}}}"#,
            r#"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"guardrails.refresh","arguments":{"check":true}}}"#,
            r#"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"guardrails.timeline","arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"guardrails.check","arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":16,"method":"tools/call","params":{"name":"guardrails.doctor","arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"guardrails.pre_work","arguments":{}}}"#,
            r#"{"jsonrpc":"2.0","id":18,"method":"ping","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":19,"method":"shutdown","params":{}}"#,
        ]
        .join("\n");
        let mut output = Vec::new();

        serve(&repo, Cursor::new(input), &mut output).expect("serve MCP input");

        let responses = response_lines(&output);
        assert_eq!(responses.len(), 20);
        assert_eq!(responses[0]["error"]["code"], -32700);
        assert_eq!(
            responses[1]["result"]["serverInfo"]["name"],
            "project-guardrails"
        );
        assert_tool_names_include(&responses[2], &["guardrails.brief", "guardrails.pre_work"]);
        assert_eq!(responses[3]["error"]["code"], -32601);
        assert_eq!(responses[4]["error"]["code"], -32600);
        assert_tool_ok(&responses[5], "schema_version");
        assert_tool_ok(&responses[6], "next_step");
        assert_tool_ok(&responses[7], "profile");
        assert_eq!(
            responses[8]["result"]["structuredContent"]["tasks"][0]["owner"],
            "MCP Unit"
        );
        assert_eq!(
            responses[9]["result"]["structuredContent"]["task"]["id"],
            "0001"
        );
        assert_eq!(
            responses[10]["result"]["structuredContent"]["task"]["status"],
            "in_progress"
        );
        assert_tool_ok(&responses[11], "handoffs");
        assert_eq!(
            responses[12]["result"]["structuredContent"]["handoff"]["slug"],
            "mcp-unit-handoff"
        );
        assert_tool_ok(&responses[13], "changed_paths");
        assert_tool_ok(&responses[14], "events");
        assert_tool_has_structured_key(&responses[15], "diagnostics");
        assert_tool_has_structured_key(&responses[16], "diagnostics");
        assert_tool_ok(&responses[17], "summary");
        assert_eq!(responses[18]["result"], json!({}));
        assert_eq!(responses[19]["result"], Value::Null);
    }

    #[test]
    fn tool_call_validation_errors_are_reported_as_json_rpc_errors() {
        let temp = TempDir::new().expect("temp dir");
        let repo = temp.path().join("mcp-validation");
        setup_repo(&repo);

        let missing_name = handle_message(
            &repo,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {}
            }),
        )
        .expect("missing name response");
        assert_eq!(missing_name["error"]["code"], -32602);
        assert!(
            missing_name["error"]["message"]
                .as_str()
                .expect("error message")
                .contains("params.name")
        );

        let invalid_status = handle_message(
            &repo,
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "guardrails.tasks.list",
                    "arguments": {
                        "status": "finished"
                    }
                }
            }),
        )
        .expect("invalid status response");
        assert_eq!(invalid_status["result"]["isError"], true);
        assert!(
            invalid_status["result"]["structuredContent"]["error"]["message"]
                .as_str()
                .expect("tool error")
                .contains("unknown task status")
        );

        assert!(
            handle_message(&repo, json!({"jsonrpc": "2.0", "result": {}})).is_none(),
            "responses from clients are ignored"
        );
    }

    fn setup_repo(repo: &Path) {
        fs::create_dir_all(repo).expect("repo dir");
        fs::write(repo.join("README.md"), "# MCP test repo\n").expect("README");
        init::run(InitArgs {
            target: repo.to_path_buf(),
            profile: "minimal".to_string(),
            profile_path: None,
            ci: Some(CiProvider::None),
            dry_run: false,
            force: false,
        })
        .expect("init repo");
        create_task(
            repo,
            TaskCreateInput {
                slug: "mcp-unit".to_string(),
                title: Some("MCP Unit".to_string()),
                priority: Some(TaskPriority::P1),
                owner: Some("MCP Unit".to_string()),
            },
        )
        .expect("create task");
    }

    fn response_lines(output: &[u8]) -> Vec<Value> {
        String::from_utf8(output.to_vec())
            .expect("utf8 output")
            .lines()
            .map(|line| serde_json::from_str(line).expect("JSON-RPC response"))
            .collect()
    }

    fn assert_tool_names_include(response: &Value, expected: &[&str]) {
        let tools = response["result"]["tools"].as_array().expect("tools array");
        for name in expected {
            assert!(
                tools.iter().any(|tool| tool["name"].as_str() == Some(name)),
                "missing tool {name}"
            );
        }
    }

    fn assert_tool_ok(response: &Value, key: &str) {
        assert_eq!(response["result"]["isError"], false);
        assert_tool_has_structured_key(response, key);
    }

    fn assert_tool_has_structured_key(response: &Value, key: &str) {
        assert!(
            response["result"]["structuredContent"].get(key).is_some(),
            "missing structuredContent key {key}: {response}"
        );
    }
}
