use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use serde_json::{Value, json};
use tempfile::TempDir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_project-guardrails"))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

#[test]
fn mcp_stdio_smoke_covers_initialize_tools_read_and_mutation() {
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path().join("mcp-smoke");
    copy_dir(&fixture_root().join("bare-repo"), &repo);

    run_guardrails(&[
        "init",
        "--target",
        repo_str(&repo),
        "--profile",
        "minimal",
        "--ci",
        "none",
    ]);
    run_guardrails(&[
        "tasks",
        "new",
        "--target",
        repo_str(&repo),
        "--slug",
        "mcp-smoke",
        "--priority",
        "p1",
    ]);

    let mut server = McpServer::start(&repo);

    let initialized = server.request(
        1,
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "project-guardrails-test",
                "version": "0"
            }
        }),
    );
    assert_eq!(
        initialized["result"]["serverInfo"]["name"],
        "project-guardrails"
    );
    assert!(initialized["result"]["capabilities"]["tools"].is_object());
    server.notify("notifications/initialized", json!({}));

    let tools = server.request(2, "tools/list", json!({}));
    let tool_names = tools["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name").to_string())
        .collect::<BTreeSet<_>>();
    for expected in [
        "guardrails.pre_work",
        "guardrails.brief",
        "guardrails.resume",
        "guardrails.status",
        "guardrails.tasks.list",
        "guardrails.tasks.get",
        "guardrails.tasks.claim",
        "guardrails.handoff.list",
        "guardrails.handoff.new",
        "guardrails.refresh",
        "guardrails.timeline",
        "guardrails.check",
        "guardrails.doctor",
    ] {
        assert!(tool_names.contains(expected), "missing MCP tool {expected}");
    }

    let brief = server.tool_call(3, "guardrails.brief", json!({}));
    assert_eq!(brief["result"]["isError"], false);
    assert_eq!(brief["result"]["structuredContent"]["schema_version"], 1);
    assert_eq!(
        brief["result"]["structuredContent"]["repo_root"],
        repo_str(&fs::canonicalize(&repo).expect("canonical repo"))
    );
    assert_eq!(
        text_content_json(&brief),
        brief["result"]["structuredContent"]
    );

    let claim = server.tool_call(
        4,
        "guardrails.tasks.claim",
        json!({
            "id": "0001",
            "owner": "MCP Smoke"
        }),
    );
    assert_eq!(claim["result"]["isError"], false);
    let task = &claim["result"]["structuredContent"]["task"];
    assert_eq!(task["id"], "0001");
    assert_eq!(task["owner"], "MCP Smoke");
    assert_eq!(task["status"], "in_progress");
    assert_eq!(task["path"], ".guardrails/state/tasks/0001-mcp-smoke.md");
}

struct McpServer {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpServer {
    fn start(repo: &Path) -> Self {
        let mut child = Command::new(binary_path())
            .args(["mcp", "serve", "--target", repo_str(repo)])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start MCP server");
        let stdin = child.stdin.take().expect("server stdin");
        let stdout = BufReader::new(child.stdout.take().expect("server stdout"));

        Self {
            child,
            stdin,
            stdout,
        }
    }

    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        writeln!(
            self.stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            })
        )
        .expect("write request");
        self.stdin.flush().expect("flush request");

        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read response");
        assert!(!line.is_empty(), "MCP server closed stdout before response");
        let response: Value = serde_json::from_str(&line).expect("valid JSON-RPC response");
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], id);
        assert!(
            response.get("error").is_none(),
            "unexpected MCP error response: {response}"
        );
        response
    }

    fn notify(&mut self, method: &str, params: Value) {
        writeln!(
            self.stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params
            })
        )
        .expect("write notification");
        self.stdin.flush().expect("flush notification");
    }

    fn tool_call(&mut self, id: u64, name: &str, arguments: Value) -> Value {
        self.request(
            id,
            "tools/call",
            json!({
                "name": name,
                "arguments": arguments
            }),
        )
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn text_content_json(response: &Value) -> Value {
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    serde_json::from_str(text).expect("text content JSON")
}

fn run_guardrails(args: &[&str]) {
    let output = Command::new(binary_path())
        .args(args)
        .output()
        .expect("run guardrails");
    assert!(
        output.status.success(),
        "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn repo_str(repo: &Path) -> &str {
    repo.to_str().expect("repo path")
}

fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination dir");

    for entry in fs::read_dir(source).expect("read fixture dir") {
        let entry = entry.expect("fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if source_path.is_dir() {
            copy_dir(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy file");
        }
    }
}
