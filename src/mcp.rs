//! A small MCP server over stdio (one JSON-RPC message per line) so an agent can drive the pointer and keyboard through Ghosthand.
//!
//! It speaks both eras of the protocol, chosen from the first message:
//!   2026-07-28   stateless: no `initialize`; every request carries `_meta["io.modelcontextprotocol/protocolVersion"]`; `server/discover` describes
//!                the server; results carry `resultType`, and list results `ttlMs` and `cacheScope`.
//!   2025-11-25   (and 2025-06-18, 2025-03-26, 2024-11-05) `initialize` negotiates a version, then `notifications/initialized`.
//! Tools: pointer (mouse and keyboard actions, behind the gate), screenshot, screen_info, gate_status. There is no tool to allow a request or to
//! switch the gate off, on purpose: answering is the person's job. Logs go to stderr, never stdout.
use std::io::{BufRead, Write};

use base64::Engine;
use serde_json::{json, Value};

use crate::config::Config;
use crate::geometry::output_info;
use crate::screen::screenshot_bytes;
use crate::session::Outcome;

pub const META: &str = "io.modelcontextprotocol/";
pub const LATEST: &str = "2026-07-28";
pub const LEGACY: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
const INSTRUCTIONS: &str = "Ghosthand drives the person's real mouse and keyboard. Take a screenshot first, work out positions in screenshot pixels, click a field before typing, then use `pointer` with a clear `reason`: the person sees a card and must allow it. If it is denied or unanswered, stop and tell them; never retry in a loop. After acting, take another screenshot to check what happened.";
const ACTIONS_HELP: &str = "jump X Y, move X Y (glides, so hover follows), click [X Y], dclick [X Y], rclick [X Y], mclick [X Y], down, up, rdown, rup, drag X1 Y1 X2 Y2, rdrag X1 Y1 X2 Y2, scroll N [X Y] (positive scrolls the content down), hscroll N [X Y], wait SECONDS, key COMBO... (e.g. ctrl+s, Return, Tab), keydown COMBO, keyup COMBO, type TEXT (\\n is Enter, \\t is Tab), paste TEXT, focus (the focused window), shot FILE [X Y W H], pos, info";

fn server_info(with_title: bool) -> Value {
    let mut v = json!({"name": "ghosthand", "version": env!("CARGO_PKG_VERSION")});
    if with_title {
        v["title"] = json!("Ghosthand");
    }
    v
}

fn hints(title: &str, read_only: bool) -> Value {
    json!({"title": title, "readOnlyHint": read_only, "destructiveHint": !read_only, "idempotentHint": read_only, "openWorldHint": !read_only})
}

pub fn tools() -> Value {
    json!([
        {
            "name": "pointer",
            "title": "Use the mouse and keyboard",
            "description": format!("Move and click the person's real mouse pointer and press keys or type on their real keyboard (click a field first: keys go to the focused window). A card appears on their screen and nothing happens until they allow it; if they deny or don't answer you get an error: stop and tell them, don't retry in a loop. Coordinates are screenshot pixels. Actions: {ACTIONS_HELP}."),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "actions": {"type": "array", "items": {"type": "string"}, "minItems": 1, "description": "One action per item, e.g. [\"move 800 40\", \"click\"]"},
                    "reason": {"type": "string", "minLength": 1, "description": "Why you need the pointer; shown on the card"},
                    "who": {"type": "string", "description": "Who is asking (default: Agent)"},
                    "countdown": {"type": "integer", "minimum": 0, "description": "Seconds the card waits (the person's setting, or longer)"}
                },
                "required": ["actions", "reason"],
                "additionalProperties": false
            },
            "outputSchema": {
                "type": "object",
                "properties": {
                    "ok": {"type": "boolean"},
                    "code": {"type": "integer", "description": "0 done, 1 failed or locked, 2 bad usage, 3 taken back, 4 denied or no answer, 5 nothing to ask with"},
                    "message": {"type": "string"},
                    "printed": {"type": "array", "items": {"type": "string"}, "description": "What pos, info and focus printed"}
                },
                "required": ["ok", "code", "message", "printed"]
            },
            "annotations": hints("Use the mouse and keyboard", false)
        },
        {
            "name": "screenshot",
            "title": "Take a screenshot",
            "description": "A PNG of the screen, or of the region [x, y, width, height] in screenshot pixels. Needs no approval.",
            "inputSchema": {"type": "object", "properties": {"region": {"type": "array", "items": {"type": "integer"}, "minItems": 4, "maxItems": 4, "description": "[x, y, width, height]"}}, "additionalProperties": false},
            "annotations": hints("Take a screenshot", true)
        },
        {
            "name": "screen_info",
            "title": "Screen size and scale",
            "description": "The output's name, size in screenshot pixels and scale.",
            "inputSchema": {"type": "object", "additionalProperties": false},
            "outputSchema": {"type": "object", "properties": {"name": {"type": "string"}, "width": {"type": "integer"}, "height": {"type": "integer"}, "scale": {"type": "number"}}, "required": ["name", "width", "height", "scale"]},
            "annotations": hints("Screen size and scale", true)
        },
        {
            "name": "gate_status",
            "title": "Gate status",
            "description": "The gate's state: idle, asking, active, allowed, denied, timeout or revoked.",
            "inputSchema": {"type": "object", "additionalProperties": false},
            "outputSchema": {"type": "object", "properties": {"state": {"type": "string"}}, "required": ["state"]},
            "annotations": hints("Gate status", true)
        }
    ])
}

fn reply(text: &str, error: bool, structured: Option<Value>) -> Value {
    let mut out = json!({"content": [{"type": "text", "text": text}], "isError": error});
    if let Some(s) = structured {
        out["structuredContent"] = s;
    }
    out
}

type RunFn = Box<dyn FnMut(&str, &str, u32, &[String]) -> Outcome>;
type ShotFn = Box<dyn Fn(Option<[i64; 4]>) -> Result<Vec<u8>, String>>;

/// What the server needs from the machine, so tests can fake it.
pub struct Backend {
    pub run: RunFn,
    pub status: Box<dyn FnMut() -> Option<String>>,
    pub info: Box<dyn Fn() -> Result<Value, String>>,
    pub screenshot: ShotFn,
}

impl Backend {
    pub fn real() -> Backend {
        Backend {
            run: Box::new(crate::app::run_actions_dyn),
            status: Box::new(|| crate::gate::default_gate(&Config::load()).and_then(|mut g| g.status())),
            info: Box::new(|| output_info(Config::load().output().as_deref()).map(|i| serde_json::from_str(&i.to_json()).unwrap_or(Value::Null))),
            screenshot: Box::new(screenshot_bytes),
        }
    }
}

/// The result of one tool call, or Err(name) for a tool that doesn't exist (a protocol error).
fn call_tool(name: &str, args: &Value, b: &mut Backend) -> Result<Value, String> {
    match name {
        "pointer" => {
            let actions: Option<Vec<String>> = args["actions"].as_array().filter(|a| !a.is_empty()).and_then(|a| a.iter().map(|x| x.as_str().map(String::from)).collect());
            let Some(actions) = actions else { return Ok(reply("actions must be a list of strings", true, None)) };
            let reason = args["reason"].as_str().unwrap_or("").trim().to_string();
            if reason.is_empty() {
                return Ok(reply("reason is required: say why you need the pointer, the person reads it on the card", true, None));
            }
            let countdown = match &args["countdown"] {
                Value::Null => 0,
                v => match v.as_u64() { Some(n) => n as u32, None => return Ok(reply("countdown must be a whole number of seconds", true, None)) },
            };
            let who = args["who"].as_str().filter(|w| !w.is_empty()).unwrap_or("Agent");
            let o = (b.run)(who, &reason, countdown, &actions);
            let text = o.printed.iter().map(String::as_str).chain(std::iter::once(o.message.as_str())).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n");
            Ok(reply(if text.is_empty() { "done" } else { &text }, o.code != 0, Some(json!({"ok": o.code == 0, "code": o.code, "message": o.message, "printed": o.printed}))))
        }
        "screenshot" => {
            let region = match &args["region"] {
                Value::Null => None,
                Value::Array(a) if a.len() == 4 && a.iter().all(|v| v.is_i64()) => Some([a[0].as_i64().unwrap_or(0), a[1].as_i64().unwrap_or(0), a[2].as_i64().unwrap_or(0), a[3].as_i64().unwrap_or(0)]),
                _ => return Ok(reply("region must be four integers: [x, y, width, height]", true, None)),
            };
            Ok(match (b.screenshot)(region) {
                Ok(png) => json!({"content": [{"type": "image", "data": base64::engine::general_purpose::STANDARD.encode(png), "mimeType": "image/png"}], "isError": false}),
                Err(e) => reply(&format!("screenshot failed: {e}"), true, None),
            })
        }
        "screen_info" => Ok(match (b.info)() {
            Ok(v) => reply(&v.to_string(), false, Some(v)),
            Err(e) => reply(&format!("couldn't read the output: {e}"), true, None),
        }),
        "gate_status" => Ok(match (b.status)() {
            Some(state) => reply(&state, false, Some(json!({"state": state}))),
            None => reply("nothing to ask with: run `ghosthand overlay`, or install zenity, yad or kdialog", true, None),
        }),
        other => Err(other.to_string()),
    }
}

fn error(id: &Value, code: i64, message: &str, data: Option<Value>) -> Value {
    let mut err = json!({"code": code, "message": message});
    if let Some(d) = data {
        err["data"] = d;
    }
    json!({"jsonrpc": "2.0", "id": id, "error": err})
}

pub struct Server {
    backend: Backend,
    /// Set by `initialize`: this process then speaks 2025-11-25 style.
    legacy: bool,
}

impl Server {
    pub fn new(backend: Backend) -> Server { Server { backend, legacy: false } }

    /// One JSON-RPC message in, the response out (None for a notification).
    pub fn handle(&mut self, message: &Value) -> Option<Value> {
        if !message.is_object() {
            return Some(error(&Value::Null, -32600, "invalid request", None));
        }
        let id = message.get("id").filter(|i| !i.is_null())?; // a notification (initialized, cancelled...): nothing to answer
        let method = message["method"].as_str().unwrap_or("");
        let params = &message["params"];
        if method == "initialize" {
            self.legacy = true;
            let asked = params["protocolVersion"].as_str().unwrap_or("");
            let version = if LEGACY.contains(&asked) { asked } else { LEGACY[0] };
            return Some(json!({"jsonrpc": "2.0", "id": id, "result": {"protocolVersion": version, "capabilities": {"tools": {"listChanged": false}}, "serverInfo": server_info(true), "instructions": INSTRUCTIONS}}));
        }
        let version = params["_meta"][format!("{META}protocolVersion")].as_str();
        if let Some(v) = version.filter(|v| *v != LATEST) {
            return Some(error(id, -32022, "Unsupported protocol version", Some(json!({"supported": [LATEST, LEGACY[0], LEGACY[1], LEGACY[2], LEGACY[3]], "requested": v}))));
        }
        if version.is_none() && !self.legacy {
            return Some(error(id, -32602, &format!("missing params._meta[{META}protocolVersion] (or call initialize first)"), None));
        }
        let modern = version.is_some();
        let result = match method {
            "server/discover" if modern => Some(json!({"supportedVersions": [LATEST], "capabilities": {"tools": {"listChanged": false}}, "instructions": INSTRUCTIONS, "_meta": {format!("{META}serverInfo"): server_info(true)}, "ttlMs": 3_600_000, "cacheScope": "public"})),
            "ping" => Some(json!({})),
            "tools/list" => {
                let mut r = json!({"tools": tools()});
                if modern {
                    r["ttlMs"] = json!(3_600_000);
                    r["cacheScope"] = json!("public");
                }
                Some(r)
            }
            "tools/call" => match call_tool(params["name"].as_str().unwrap_or(""), &params["arguments"], &mut self.backend) {
                Ok(r) => Some(r),
                Err(name) => return Some(error(id, -32602, &format!("Unknown tool: {name}"), None)),
            },
            _ => None,
        };
        let Some(mut result) = result else { return Some(error(id, -32601, &format!("method not found: {method}"), None)) };
        if modern {
            result["resultType"] = json!("complete");
            let meta = result.get("_meta").cloned().unwrap_or_else(|| json!({}));
            let mut meta = meta;
            if meta.get(format!("{META}serverInfo")).is_none() {
                meta[format!("{META}serverInfo")] = server_info(false);
            }
            result["_meta"] = meta;
        }
        Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
    }
}

/// Serve on stdin and stdout until stdin closes (the shutdown signal).
pub fn serve<R: BufRead, W: Write>(input: R, mut out: W, backend: Backend) -> i32 {
    let mut server = Server::new(backend);
    for line in input.lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(line) {
            Ok(v) => server.handle(&v),
            Err(_) => Some(error(&Value::Null, -32700, "parse error", None)),
        };
        if let Some(r) = reply {
            if writeln!(out, "{r}").and_then(|_| out.flush()).is_err() {
                break;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend() -> Backend {
        Backend {
            run: Box::new(|_, _, _, _| Outcome::default()),
            status: Box::new(|| Some("idle".into())),
            info: Box::new(|| Ok(json!({"name": "X", "width": 100, "height": 50, "scale": 1.0}))),
            screenshot: Box::new(|_| Ok(vec![1, 2, 3])),
        }
    }

    fn modern(method: &str, params: Value, version: &str) -> Value {
        let mut p = params;
        p["_meta"] = json!({format!("{META}protocolVersion"): version, format!("{META}clientCapabilities"): {}});
        json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": p})
    }

    fn ask(s: &mut Server, method: &str) -> Value { s.handle(&modern(method, json!({}), LATEST)).unwrap() }

    #[test]
    fn discover_describes_the_server() {
        let r = &ask(&mut Server::new(backend()), "server/discover")["result"];
        assert_eq!(r["resultType"], "complete");
        assert_eq!(r["supportedVersions"], json!([LATEST]));
        assert!(r["capabilities"]["tools"].is_object() && r["ttlMs"].is_number());
        assert!(["public", "private"].contains(&r["cacheScope"].as_str().unwrap()));
        assert_eq!(r["_meta"][format!("{META}serverInfo")]["name"], "ghosthand");
    }

    #[test]
    fn every_result_says_complete_and_carries_server_info() {
        let r = &ask(&mut Server::new(backend()), "tools/list")["result"];
        assert_eq!(r["resultType"], "complete");
        assert_eq!(r["_meta"][format!("{META}serverInfo")]["name"], "ghosthand");
        assert!(r["ttlMs"].is_number());
    }

    #[test]
    fn a_request_without_meta_is_malformed() {
        let r = Server::new(backend()).handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})).unwrap();
        assert_eq!(r["error"]["code"], -32602);
    }

    #[test]
    fn an_unknown_version_gets_the_supported_list() {
        let r = Server::new(backend()).handle(&modern("tools/list", json!({}), "1900-01-01")).unwrap();
        assert_eq!(r["error"]["code"], -32022);
        assert_eq!(r["error"]["data"]["requested"], "1900-01-01");
        let supported = r["error"]["data"]["supported"].as_array().unwrap();
        assert!(supported.contains(&json!(LATEST)) && supported.contains(&json!("2025-11-25")));
    }

    #[test]
    fn unknown_methods_and_tools_are_protocol_errors() {
        let mut s = Server::new(backend());
        assert_eq!(ask(&mut s, "nope")["error"]["code"], -32601);
        let r = s.handle(&modern("tools/call", json!({"name": "rm", "arguments": {}}), LATEST)).unwrap();
        assert_eq!(r["error"]["code"], -32602);
    }

    #[test]
    fn notifications_get_no_answer() {
        assert!(Server::new(backend()).handle(&json!({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": {"requestId": 1}})).is_none());
    }

    #[test]
    fn initialize_negotiates_the_version_and_later_requests_need_no_meta() {
        let mut s = Server::new(backend());
        let r = s.handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}}})).unwrap();
        assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(r["result"]["serverInfo"]["name"], "ghosthand");
        let newer = s.handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "initialize", "params": {"protocolVersion": "2030-01-01"}})).unwrap();
        assert_eq!(newer["result"]["protocolVersion"], LEGACY[0], "an unknown version gets the newest one we know");
        let list = s.handle(&json!({"jsonrpc": "2.0", "id": 3, "method": "tools/list"})).unwrap();
        assert!(list["result"].get("resultType").is_none());
        let names: Vec<&str> = list["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["pointer", "screenshot", "screen_info", "gate_status"]);
        assert_eq!(s.handle(&json!({"jsonrpc": "2.0", "id": 4, "method": "ping"})).unwrap()["result"], json!({}));
    }

    #[test]
    fn every_tool_is_well_formed_and_the_order_is_stable() {
        for t in tools().as_array().unwrap() {
            let name = t["name"].as_str().unwrap();
            assert!(!name.is_empty() && name.len() <= 128 && name.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c)));
            assert_eq!(t["inputSchema"]["type"], "object");
            assert!(t["title"].is_string() && t["description"].is_string());
            if t.get("outputSchema").is_some() {
                assert_eq!(t["outputSchema"]["type"], "object");
            }
            for h in ["readOnlyHint", "destructiveHint", "idempotentHint", "openWorldHint"] {
                assert!(t["annotations"][h].is_boolean(), "{name} {h}");
            }
        }
    }

    #[test]
    fn the_pointer_acts_and_the_rest_are_read_only_and_there_is_no_way_to_approve() {
        let tools = tools();
        let ro: Vec<(String, bool)> = tools.as_array().unwrap().iter().map(|t| (t["name"].as_str().unwrap().to_string(), t["annotations"]["readOnlyHint"].as_bool().unwrap())).collect();
        assert_eq!(ro, [("pointer".to_string(), false), ("screenshot".into(), true), ("screen_info".into(), true), ("gate_status".into(), true)]);
        for t in tools.as_array().unwrap() {
            assert!(!t["description"].as_str().unwrap().to_lowercase().contains("approve"));
            assert!(t["inputSchema"]["properties"].get("mode").is_none());
        }
    }

    fn call(name: &str, args: Value, b: &mut Backend) -> Value { call_tool(name, &args, b).unwrap() }

    #[test]
    fn pointer_needs_actions_and_a_reason() {
        let mut b = backend();
        for args in [json!({"reason": "x"}), json!({"actions": ["click"], "reason": "  "}), json!({"actions": [1], "reason": "x"}), json!({"actions": ["click"], "reason": "x", "countdown": "soon"}), json!({"actions": [], "reason": "x"})] {
            assert_eq!(call("pointer", args.clone(), &mut b)["isError"], true, "{args}");
        }
    }

    #[test]
    fn pointer_passes_who_reason_and_actions_and_reports_structured_output() {
        let seen = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
        let s2 = seen.clone();
        let mut b = backend();
        b.run = Box::new(move |who, reason, countdown, actions| {
            s2.borrow_mut().push((who.to_string(), reason.to_string(), countdown, actions.to_vec()));
            Outcome { code: 4, message: "Ghosthand: not allowed (denied).".into(), printed: vec![] }
        });
        let r = call("pointer", json!({"actions": ["move 1 2", "click"], "reason": "testing", "who": "Claude", "countdown": 5}), &mut b);
        assert_eq!(seen.borrow()[0], ("Claude".to_string(), "testing".to_string(), 5, vec!["move 1 2".to_string(), "click".to_string()]));
        assert_eq!(r["isError"], true);
        assert_eq!(r["structuredContent"], json!({"ok": false, "code": 4, "message": "Ghosthand: not allowed (denied).", "printed": []}));
        assert!(r["content"][0]["text"].as_str().unwrap().contains("denied"));
    }

    #[test]
    fn success_returns_what_pos_printed() {
        let mut b = backend();
        b.run = Box::new(|_, _, _, _| Outcome { code: 0, message: String::new(), printed: vec!["{\"x\": 1}".into()] });
        let r = call("pointer", json!({"actions": ["pos"], "reason": "x"}), &mut b);
        assert_eq!(r["isError"], false);
        assert!(r["content"][0]["text"].as_str().unwrap().contains("\"x\": 1"));
    }

    #[test]
    fn screenshot_returns_an_image_and_a_bad_region_is_a_tool_error() {
        let mut b = backend();
        let r = call("screenshot", json!({}), &mut b);
        assert_eq!(r["content"][0]["type"], "image");
        assert_eq!(r["content"][0]["mimeType"], "image/png");
        assert_eq!(r["content"][0]["data"], "AQID");
        assert_eq!(call("screenshot", json!({"region": [1, 2, 3]}), &mut b)["isError"], true);
    }

    #[test]
    fn screen_info_and_gate_status_have_structured_output() {
        let mut b = backend();
        assert_eq!(call("screen_info", json!({}), &mut b)["structuredContent"]["width"], 100);
        let r = call("gate_status", json!({}), &mut b);
        assert_eq!(r["structuredContent"], json!({"state": "idle"}));
        assert_eq!(r["content"][0]["text"], "idle");
        b.status = Box::new(|| None);
        assert_eq!(call("gate_status", json!({}), &mut b)["isError"], true);
    }

    #[test]
    fn odd_messages_never_panic() {
        let mut s = Server::new(backend());
        let odd = [json!(null), json!(1), json!("x"), json!([]), json!({}), json!({"id": 1}), json!({"id": 1, "method": 5}), json!({"id": {"a": 1}, "method": "tools/call", "params": 7}),
            json!({"id": 1, "method": "tools/call", "params": {"name": 3}}), json!({"id": 1, "method": "tools/call", "params": {"name": "pointer", "arguments": [1]}}),
            json!({"id": 1, "method": "initialize", "params": {"protocolVersion": 5}}), json!({"id": 2, "method": "tools/list", "params": {"_meta": 4}})];
        for m in &odd {
            let _ = s.handle(m);
        }
    }

    #[test]
    fn the_loop_answers_line_by_line_and_stops_at_eof() {
        let input = format!("{}\n\nnot json\n{}\n", modern("tools/list", json!({}), LATEST), json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        let mut out = Vec::new();
        assert_eq!(serve(input.as_bytes(), &mut out, backend()), 0);
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["result"]["resultType"], "complete");
        assert_eq!(lines[1]["error"]["code"], -32700);
    }
}
