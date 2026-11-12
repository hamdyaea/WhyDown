# WhyDown JSON report schema v2

WhyDown v0.3.0 emits JSON schema **2** (`--format json`). This is a breaking change from schema 1.
Consumers must check `schema_version` and should compare the same `focus` and `lookback_seconds`.

## Fields

- `schema_version`: integer, `2`
- `tool`, `version`, `generated_at` (UTC ISO 8601), `lookback_seconds`, `focus`, `demo`
- `summary`: `warning_count`, `info_count`, `current_warnings`, `historical_warnings`, `active_connectivity_probed` (always `false`)
- `system`: OS/kernel, uptime, routes, local DNS configuration, failed systemd units, interfaces with counters and optional `operstate`, disks and optional directory breakdowns
- `findings`: `id`, `severity`, `confidence`, `scope`, `title`, `explanation`, `evidence`, `checks`
- `network_incidents`: historical event clusters, including `subject` (identified network interface, or `null`) and start/end times
- `timeline`: `timestamp`, `kind`, `label`, `subject` and `source`; optional subject/source are `null` when unknown
- `collection_notes`: permission, missing-command and journal-truncation caveats

## Temporal scope and confidence

- `scope=current_snapshot`: a local state observed at collection time. Not proof of internet connectivity.
- `scope=historical_journal`: a past classified journal message, not proof a problem still exists.
- `scope=unverified`: no rule matched. This is not a health guarantee.

A kernel interface `operstate=up` and the existence of default routes do **not** verify internet reachability.
The engine never probes network endpoints and does not monitor continuously.

`confidence=observed` means a value or matching log pattern was observed, **not** that the inferred root cause is proven.
`network_incidents` are 120-second time groups, separated by interface identifier when known; they are not validated outage periods.

## Privacy and schema compatibility

Only short, ASCII-validated program, network-interface or systemd-unit identifiers are extracted from matching journal entries. No raw journal line is kept. The output may nevertheless contain mountpoints, names of local units/interfaces and other configuration identifiers; review before sharing.

Do not silently parse schema 2 as schema 1. For distribution comparisons, use finding IDs, scopes, summaries and limitations rather than timestamps or hostnames.


## Additions in WhyDown v0.3.1 (schema still 2)

- `system.system_units_checked`: boolean; false means the system manager was not checked successfully.
- `system.failed_user_units`: array of currently failed unit names from the **invoking user's** manager, or `null` if that manager could not be queried. An empty array means the query succeeded and found no failed units.
- `service_event_windows`: historical five-second journal message clusters, each with `start`, `end`, `entry_count`, `unattributed_entries`, `units`, and `status`. **A cluster is not an independent failed-service count.**
- `findings[].id=FAILED_USER_SYSTEMD_UNITS` signals current user-unit failures; `SERVICE_FAILURE_LOGS` remains a historical journal observation.

A service that exited with code 1 may have automatically restarted, and a SIGTERM signal may occur during shutdown. Check the surrounding journal and the present system/user manager states before interpreting these observations. Reports never establish that a service is currently stopped simply because it failed historically.
