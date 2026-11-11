# Demo reports

Generate fresh reports using the built executable:

```bash
cargo run -- demo --format text
cargo run -- demo --format json --output example.json
cargo run -- demo --format html --output example.html
```

The demo dataset is fictional and its timestamps are calculated relative to the time of execution. This avoids committing sample outputs that look like real system telemetry.
