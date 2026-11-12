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


## Additions in WhyDown v0.3.2 (schema still 2)

- `service_event_windows[].boot_relation`: `before_current_boot_estimate`, `since_current_boot_estimate`, `spans_boot_estimate` or `unknown`. Derived from report generation time minus `system.uptime_seconds`; this is not a true journal boot ID. Time synchronization and wall-clock changes can affect the estimate.
- Privilege escalation audit records can contain command-line arguments with diagnostic phrases. Such command text is not a genuine failed service/network/storage event and is excluded from classification when identified as an audit command record.
- `SERVICE_FAILURE_LOGS` evidence may mention how many classified messages predate the estimated current boot.

All extensions are additive; `schema_version` remains 2.


## Additions in WhyDown v0.3.6 (schema remains 2)

For `focus=system` or `focus=all`, the `system` object includes:
- `dpkg_checked`: `true` when the local dpkg status database was queried successfully, `false` if present but query failed, `null` if the dpkg database is absent or the focus excludes system.
- `dpkg_problem_count`: number of incomplete/broken dpkg states (integer when queried successfully; `null` otherwise).
- `dpkg_problems`: bounded list (max 50) of `{name, state, reinstall_required}`; these are local package identifiers, not raw logs.

`DPKG_INCOMPLETE_PACKAGES` is a warning with `scope=current_snapshot` when a dpkg query detects incomplete configurations. Only the first 15 affected package names are included in finding evidence. The collector ignores harmless residual configuration files and fully installed packages.

`dpkg-query` is a read-only database inspection; WhyDown does not execute a repair, simulate APT, or establish what caused a package's post-install failure.


## Additions in WhyDown v0.4.0 (schema remains 2)

Top-level `journal_current_boot_only` (boolean) indicates whether `journalctl --boot=0` was requested. Journal entries are still constrained by `lookback_seconds`; route, filesystem, dpkg and systemd status snapshots reflect the present and are not restricted by journal boot selection.

`--exit-on-warning` is a CLI process status policy, not a JSON field. Exit codes: 0 for successful report generation (including warnings without that flag), 1 for warnings **with** the flag, and 2 for CLI/report write errors. Successful report generation does not guarantee a healthy host.

Collector processes are time-bounded and stdout-limited. Collection failures, timeouts and oversized outputs must appear in `collection_notes` and should not be interpreted as a healthy subsystem. File output uses private Unix permissions, and `--overwrite` uses staged replacement.


## Additions in WhyDown v0.4.1 (schema remains 2)

Under `system`:
- `pacman_checked`: `null` when the pacman local database is not found (or outside system focus); `false` if found but querying was unavailable; `true` after `pacman -Dk` returned a status.
- `pacman_consistent`: `null` when unchecked, `true` for `pacman -Dk` success and `false` for a nonzero command status.

A nonzero command status emits `PACMAN_DATABASE_CHECK_FAILED` with `scope=current_snapshot` and `confidence=limited`. This does not prove the database is corrupt, all installed files exist or that the machine is healthy. dpkg and pacman databases are checked independently where present.


## Additions in WhyDown v0.5.0 (schema remains 2)

The `system` object adds:
- `memory`: object `{total_kib, available_kib, swap_total_kib, swap_free_kib}` or null if host memory info is unavailable.
- `pressure`: array of optional CPU/memory/I/O measurements `{resource, some_avg60_percent_x100, full_avg60_percent_x100}`. Numbers represent *hundredths of one percent* of task stall time averaged over 60 seconds. CPU full may be null as it is not necessarily available.
- `file_handles`: `{allocated, unused, maximum}` (kernel global `file-nr`) or null when unsupported. Subtract unused from allocated for estimated in-use count.
- `inodes`: array of `{mountpoint, used_percent}` for the writable-capable mounts also recognized by the disk collector.
- `md_arrays`: array of `{name, expected, active}` for Linux MD arrays with explicit member counts.

The corresponding new warning IDs are `LOW_MEMORY_HEADROOM`, `MEMORY_STALL_PRESSURE`, `IO_STALL_PRESSURE`, `FILE_HANDLE_LIMIT_NEAR`, `INODE_NEAR_FULL` and `MD_RAID_DEGRADED`. All have `scope=current_snapshot`. An absent sensor is not evidence of a clean result; the fields are null or empty on unsupported hosts. Existing JSON fields are retained; version 2 remains additive.
