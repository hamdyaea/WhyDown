# Contributing to WhyDown

Thanks for improving WhyDown! The priority is evidence quality, low complexity and safe behavior.

## Development setup

```bash
cargo fmt --all
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

## Rules for contributions

1. Keep diagnostics read-only by default. Never execute arbitrary journal text, log content or external shell snippets.
2. Classify what is **observed** separately from what is **hypothesized**.
3. Provide supporting evidence, verification steps, and at least one unit/integration regression test for each rule.
4. Include realistic, anonymized fixtures; never commit raw private logs, tokens, IPs or user data.
5. Preserve JSON schema backwards compatibility when possible, and bump `schema_version` for breaking changes.
6. No telemetry, outbound requests or cloud dependency without an explicit user-controlled opt-in and documented design.
7. Use small, understandable Rust modules; third-party dependencies require justification.

For bug reports, include OS, kernel, expected result, observed result, exact CLI options and anonymized sample inputs. Open an issue before very large changes.

## License

Contributions are provided under GPL-3.0-only. By submitting a contribution you represent that you have the rights necessary to license it under this license.
