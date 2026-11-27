## What changed

<!-- A sentence or two. Why, not just what. -->

## How I tested it

<!-- cargo test is not enough on its own for anything that touches the gate, the devices or the keyboard. Say what you ran. -->

## Checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy --all-targets -- -D warnings` is quiet
- [ ] I added a test for the change (or said why not)
- [ ] The README, `docs/` and the changelog are updated if behavior changed
- [ ] If this touches the devices or the daemon: I ran `examples/live_test`
- [ ] It doesn't weaken the gate (see "Rules that don't bend" in CONTRIBUTING.md)
