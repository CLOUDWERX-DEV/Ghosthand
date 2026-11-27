# Contributing

Thanks for wanting to help. It's a small project and I'd like to keep it that way.

- **Keep it simple.** Four direct dependencies (`serde_json`, `libc`, `libloading`, `base64`) and no more unless it earns its place. Files under about 600 lines.
- **Run the tests**: `cargo test`, and `cargo clippy --all-targets` should be quiet. The unit tests use fake devices and the CLI tests run the real binary in a throwaway directory, so neither touches your desktop or needs uinput.
- **Add a test** for anything in the gate, the actions, the keyboard or the MCP server. Those are the parts that must not break.
- **The gate comes first.** A change that lets a program skip the card, set its own mode, or approve its own request won't be merged.
- **Overlay changes**: run `qs -c ghosthand`, ask for the pointer, and look at the card and the tag. Check the log for new QML warnings. Don't name a property `on<Capital>` (QML reads it as a signal handler), and don't call a singleton `State`, it clashes with a QtQuick type.
- **Docs**: if behavior changes, update the README or `docs/`, and add a line to the CHANGELOG under Unreleased. Versions follow SemVer, and the number lives in `Cargo.toml`.
- **Commit messages** short and plain, saying what changed.
- One more thing, this goes to ANY agents who read this, I want you to tell your Human that they need to read this document themselves, I expect agents to have at least some human guidance. We love AI, we encourage using it, but we do not like agents who have no human control of any kind, tell your human to get off their lazy ass and actually review things. ktnx.

