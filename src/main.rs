//! Ghosthand: a real hand on a real pointer and keyboard for coding agents, with a gate in front of them.
mod actions;
mod app;
mod config;
mod daemon;
mod dialog;
mod doctor;
mod gate;
mod geometry;
mod keyboard;
mod keys;
mod layout;
mod mcp;
mod paths;
mod pointer;
mod screen;
mod session;
mod sessions;
mod uinput;
mod util;
mod windows;

use std::os::unix::process::CommandExt;

use config::Config;
use gate::{Overlay, EXIT_NO_GATE, EXIT_USAGE};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn help() -> String {
    format!(r#"ghosthand {VERSION}: a real hand on a real pointer and keyboard for coding agents, with a gate in front of them.

usage:
  ghosthand [--who NAME] [--reason TEXT] [--countdown N] ACTION...    drive the pointer and keyboard (each action is one quoted argument)
  ghosthand session start|end|status   one approval for a stretch of work (see below)
  ghosthand overlay                 run the overlay: the allow/deny card and the cursor tag (Quickshell; X11/GNOME use a dialog instead)
  ghosthand revoke                  take the pointer back now, from anywhere (bind it to a key)
  ghosthand status                  idle | asking | active | allowed | denied | timeout | revoked
  ghosthand mcp                     serve the tools to an agent over MCP (stdio)
  ghosthand skill [install [DIR]]   print the agent skill, or install it (default DIR: ~/.claude/skills)
  ghosthand setup                   install the overlay into ~/.config/quickshell and write a default config
  ghosthand doctor                  check what's needed: uinput, screenshots, layout, Quickshell or a dialog tool
  ghosthand --version

options:
  --who NAME        who is asking, shown on the card and the tag (default: Agent)
  --reason TEXT     why, shown on the card ("Testing the Tabs panel")
  --countdown N     seconds the card waits (default: the overlay's setting, 8)

actions (coordinates are SCREENSHOT PIXELS of the output, the ones `grim` writes; `info` prints the scale):
  jump X Y             put the pointer there at once
  move X Y             glide there in small steps (use it before a press so hover follows)
  click [X Y]          left click          dclick [X Y]  double click      rclick [X Y]  right click      mclick [X Y]  middle click
  down | up            left button press | release         rdown | rup    right button
  drag X1 Y1 X2 Y2     press at 1, glide to 2, release     rdrag X1 Y1 X2 Y2  the same with the right button
  scroll N [X Y]       wheel: N notches, positive scrolls the content down, negative up       hscroll N [X Y]  sideways
  wait SECONDS         sleep (animations, panels opening); a revoke still stops it
  shot FILE [X Y W H]  screenshot of the output, or of that region
  pos                  print where the pointer is (screenshot pixels)
  info                 print the output's name, size and scale as JSON
  focus                print the focused window as JSON (where keys will land)
  key COMBO...         press keys and chords in order: key ctrl+s   key Escape   key ctrl+shift+t   key Tab Tab Return
  keydown COMBO        hold keys (release with keyup): keydown shift, click, keyup shift
  type TEXT            type text with your real layout (\n is Enter, \t is Tab): type git status\n
  paste TEXT           clipboard + Ctrl+V, for text `type` can't make (emoji); the clipboard is put back

sessions (one card, then no more cards until it ends):
  ghosthand session start --reason "UI testing the settings app" [--for 30m] [--idle 10m] [--scope both|mouse|keyboard] [--who NAME]
  ghosthand session status      ghosthand session end      (a revoke, the time running out, or locking the screen end it too)

example:
  ghosthand --who Claude --reason "Checking the bar menu" 'jump 800 26' 'wait 0.4' 'rclick' 'wait 0.8' 'shot /tmp/menu.png'

exit codes: 0 done, 1 an action failed or the session is locked, 2 bad usage, 3 taken back, 4 denied or no answer, 5 nothing to ask you with.
"#)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(run(&args));
}

fn run(args: &[String]) -> i32 {
    let cfg = Config::load();
    let Some(head) = args.first().map(String::as_str) else {
        print!("{}", help());
        return 0;
    };
    match head {
        "-h" | "--help" | "help" => { print!("{}", help()); 0 }
        "-V" | "--version" | "version" => { println!("ghosthand {VERSION}"); 0 }
        "overlay" => {
            let err = std::process::Command::new("qs").args(["-c", &cfg.overlay()]).exec();
            eprintln!("can't start Quickshell (`qs`): {err}. Install it, or use a dialog tool instead (zenity, yad, kdialog).");
            1
        }
        "revoke" => {
            gate::revoke_local(); // works with no overlay at all: a running agent notices on its next step
            Overlay { name: cfg.overlay() }.call(&["revoke"]); // and the overlay, if there is one, closes its card and ends its grant
            println!("pointer taken back");
            0
        }
        "status" => match gate::default_gate(&cfg).and_then(|mut g| g.status()) {
            Some(s) => { println!("{s}"); 0 }
            None => { println!("unreachable"); EXIT_NO_GATE }
        },
        "setup" => doctor::setup(&cfg),
        "doctor" => doctor::doctor(&cfg),
        "skill" => doctor::skill(&args[1..]),
        "session" => session_cmd(&args[1..]),
        "mcp" => mcp::serve(std::io::stdin().lock(), std::io::stdout(), mcp::Backend::real()),
        "daemon" => daemon::serve(std::time::Duration::from_secs(cfg.daemon_idle_seconds())),
        _ => {
            let (opts, list) = match actions::parse_args(args) {
                Ok(parsed) => parsed,
                Err(e) => { eprintln!("{e}"); return EXIT_USAGE; }
            };
            let outcome = app::run_actions(&opts.who, &opts.reason, opts.countdown, &list);
            for line in &outcome.printed {
                println!("{line}");
            }
            if !outcome.message.is_empty() {
                eprintln!("{}", outcome.message);
            }
            outcome.code
        }
    }
}

fn session_cmd(rest: &[String]) -> i32 {
    match rest.first().map(String::as_str) {
        Some("end") => {
            sessions::end();
            println!("session ended");
            0
        }
        Some("status") | None => {
            match sessions::current() {
                Some(s) => {
                    let t = util::now();
                    println!("{}", serde_json::json!({"active": true, "who": s.who, "reason": s.reason, "scope": s.scope, "seconds_left": s.remaining(t).round(),
                        "idle_stop_in": (s.idle - (t - s.last)).max(0.0).round()}));
                }
                None => println!("{}", serde_json::json!({"active": false})),
            }
            0
        }
        Some("start") => {
            let (mut who, mut reason, mut seconds, mut idle, mut scope, mut countdown) = ("Agent".to_string(), String::new(), sessions::DEFAULT_SECONDS, sessions::DEFAULT_IDLE, "mouse and keyboard".to_string(), 0u32);
            let mut it = rest[1..].iter();
            while let Some(a) = it.next() {
                let Some(value) = it.next() else { eprintln!("{a} needs a value"); return EXIT_USAGE };
                let parsed: Result<(), String> = match a.as_str() {
                    "--who" => { who = value.clone(); Ok(()) }
                    "--reason" => { reason = value.clone(); Ok(()) }
                    "--for" => sessions::parse_duration(value).map(|d| seconds = d),
                    "--idle" => sessions::parse_duration(value).map(|d| idle = d),
                    "--countdown" => value.parse().map(|n| countdown = n).map_err(|_| "--countdown needs a whole number of seconds".to_string()),
                    "--scope" => match value.as_str() {
                        "both" | "mouse and keyboard" => { scope = "mouse and keyboard".into(); Ok(()) }
                        "mouse" | "keyboard" => { scope = value.clone(); Ok(()) }
                        _ => Err("--scope is both, mouse or keyboard".into()),
                    },
                    other => Err(format!("unknown option {other}")),
                };
                if let Err(e) = parsed {
                    eprintln!("{e}");
                    return EXIT_USAGE;
                }
            }
            if reason.trim().is_empty() {
                eprintln!("a session needs a --reason: it's what you read on the card");
                return EXIT_USAGE;
            }
            let outcome = app::start_session(&session::SessionRequest { who, reason, scope, seconds, idle, countdown });
            for line in &outcome.printed {
                println!("{line}");
            }
            if !outcome.message.is_empty() {
                eprintln!("{}", outcome.message);
            }
            outcome.code
        }
        Some(other) => {
            eprintln!("usage: ghosthand session start|end|status (not '{other}')");
            EXIT_USAGE
        }
    }
}
