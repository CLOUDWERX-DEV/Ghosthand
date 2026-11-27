# Contributing

Thanks for wanting to help. It's a small project and I'd like to keep it that way.

- **Keep it simple.** No Python dependencies, and files under about 600 lines.
- **Run the tests**: `python -m unittest discover -s tests -t . -v`. They use a fake pointer, so uinput isn't needed.
- **Add a test** for anything in the gate, the actions or the MCP server. Those are the parts that must not break.
- **The gate comes first.** A change that lets a program skip the card, set its own mode, or approve its own request won't be merged.
- **Overlay changes**: run `qs -c ghosthand`, ask for the pointer, and look at the card and the tag. Check the log for new QML warnings. Don't name a property `on<Capital>` (QML reads it as a signal handler), and don't call a singleton `State`, it clashes with a QtQuick type.
- **Docs**: if behavior changes, update the README or `docs/`, and add a line to the CHANGELOG under Unreleased. Versions follow SemVer, and the number lives in `ghosthand/__init__.py`.
- **Commit messages** short and plain, saying what changed.
