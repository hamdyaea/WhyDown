# Nagios / Icinga monitoring with WhyDown

WhyDown v0.12.0 includes a **native, dependency-free monitoring-plugin mode**
for Linux. Run local checks without running the full forensic/journal scan.
Checks are read-only and do not send any data to the Internet, query a remote
endpoint, restart services, or run as root by default.

WhyDown provides two complementary graphing workflows:

1. **Nagios performance data**, compatible with Nagios Core / Icinga and
   graph collectors such as PNP4Nagios. Nagios performs repeated checks and
   a configured perfdata backend stores and graphs the resulting values.
2. **Standalone local graphs**, with `--record` to append explicitly
   requested time-stamped samples, `graph` to produce a self-contained SVG
   time-series plot, and `dashboard` to generate a complete offline HTML
   page containing charts for every recorded metric.

Nagios Core does **not** itself store the perfdata as a time series or graph
it: install/configure a perfdata processor if you want charts inside Nagios.
Official references:
- [Monitoring Plugins API and performance-data syntax](https://www.monitoring-plugins.org/doc/guidelines.html)
- [Nagios Core performance graphs using PNP4Nagios](https://library.nagios.com/docs/nagios-core/configuration/Nagios-Core-Performance-Graphs-Using-PNP4Nagios)

## Quick start

Install normally (static Linux binary; Rust/Python/Qt not needed):

```bash
sh scripts/install.sh
~/.local/bin/whydown nagios --help
```

Test the plugin and return status:

```bash
~/.local/bin/whydown nagios check --metric memory --warning 80 --critical 95
echo "Nagios status: $?"
~/.local/bin/whydown nagios check --metric disk --path / --warning 85 --critical 95
~/.local/bin/whydown nagios check --metric failed-units
~/.local/bin/whydown nagios check --metric all --timeout 8
```

Sample (fictional output; actual status and values depend on the host):

```text
WHYDOWN WARNING - disk 88.00% (...) | 'disk_used_pct'=88.00%;80;90;0;100
```

| Exit code | Plugin state | Meaning |
| --- | --- | --- |
| 0 | OK | Check succeeded and no configured threshold fired |
| 1 | WARNING | The warning threshold fired |
| 2 | CRITICAL | The critical threshold fired |
| 3 | UNKNOWN | Input invalid, sensor unavailable, command timed out, or recording failed |

**Unavailable data is never interpreted as zero or healthy.** The aggregated
`all` mode uses the most severe observed state, giving real CRITICAL/WARNING
readings precedence while also identifying UNKNOWN samples in its summary.

The plugin writes **one status line to stdout** containing the canonical
`'label'=valueUOM;warn;crit;min;max` performance-data format. No WD ASCII
banner, ANSI escapes, progress bar or diagnostic JSON is mixed into this output.
Use `--verbose` only when longer multi-line plugin output is acceptable.

## Metrics and default thresholds

| `--metric` | Perfdata label | Measurement | Warn / Crit |
| --- | --- | --- | --- |
| memory | mem_used_pct | 100 × (MemTotal − MemAvailable) / MemTotal | 80 / 95% |
| swap | swap_used_pct | Percent of swap used; 0% if none configured | 70 / 90% |
| disk | disk_used_pct | Percent used on selected **local** filesystem | 80 / 90% |
| inodes | inode_used_pct | Percent used inodes on selected local filesystem | 80 / 95% |
| load | load1_per_cpu_pct | 1-minute load / logical CPUs × 100 (not actual CPU usage) | 85 / 150% |
| cpu-psi | cpu_psi_some_pct | CPU PSI `some avg60`, a 60-second stall average | 50 / 90% |
| memory-psi | mem_psi_full_pct | Memory PSI `full avg60` | 5 / 15% |
| io-psi | io_psi_full_pct | I/O PSI `full avg60` | 10 / 20% |
| file-handles | fd_used_pct | Allocated-in-use / maximum system file handles | 80 / 95% |
| route | default_route | 1 if either IPv4/IPv6 default route exists, else 0 | Warn if absent |
| dns | dns_servers | Count of valid `nameserver` entries in resolv.conf | Warn if zero |
| failed-units | failed_units | Currently failed systemd units | Warn at 1–2; critical ≥3 |

Thresholds are conservative defaults and should be adjusted to the host's
purpose. CPU PSI is **not** CPU utilization. Load normalized to core count can
exceed 100%. DNS configuration and default route are **not connectivity
probes**. `failed-units` is a live systemd status, not a historical event count.

`--metric all` checks all listed metrics. Custom ranges require a single
metric. `--path /srv` applies only to the disk/inode checks (or to both in
`all` mode); GNU `df` is invoked without the shell, with a short timeout and
local-filesystem filtering. Avoid setting the path to remote mounts.

## Official threshold ranges

Standard Nagios ranges alert when outside the supplied range, unless `@`
inverts the test:

- `80` — alert for values outside `0..80` (including values below 0)
- `0:90` — alert outside `0..90`
- `10:` — alert below 10
- `~:10` — alert above 10, without a lower bound
- `@10:20` — alert between 10 and 20 **inclusive**

For example, a memory alert over 90% and a critical alert over 97%:

```bash
whydown nagios check --metric memory --warning 90 --critical 97
```

## Built-in graphing and offline dashboard

An ordinary check is read-only and never stores data. For **real charts**
measurements must be gathered over time (cron, systemd timer, Nagios, etc.).
You opt into local recording explicitly:

```bash
mkdir -p -m 700 "$HOME/.local/state/whydown"
whydown nagios check --metric all \
  --record "$HOME/.local/state/whydown/nagios.tsv"
```

Repeat the same command regularly. Then generate graphs:

```bash
whydown nagios graph \
  --history "$HOME/.local/state/whydown/nagios.tsv" \
  --metric memory --hours 24 \
  --output "$HOME/.local/state/whydown/memory.svg" --overwrite

whydown nagios dashboard \
  --history "$HOME/.local/state/whydown/nagios.tsv" \
  --hours 168 \
  --output "$HOME/.local/state/whydown/dashboard.html" --overwrite
```

Open `memory.svg` or `dashboard.html` locally in a browser. Each chart uses
**actual recorded timestamps and samples**, not fictional trends. Samples are
bounded and downsampled to preserve extrema. Without enough distinct
timestamps, the graph explicitly says there is no trend yet.

There is no HTTP server, JavaScript bundle, database, network probe or
scheduler in this feature. History is stored in a private, append-only TSV
file (**0600**) only when `--record` is passed. Chart exports are separate
0600 files and are not overwritten without `--overwrite`. The dashboard
marks fictional `--demo` samples as demo data. The history reader limits
file size and row count to prevent oversized report processing.

**Suggested cron entry**, every five minutes (adapt absolute executable and
home-directory paths for your Nagios service user):

```cron
*/5 * * * * /home/YOUR_USER/.local/bin/whydown nagios check --metric all --record /home/YOUR_USER/.local/state/whydown/nagios.tsv >/dev/null
```

For production integration, use Nagios's regular scheduler and **perfdata
processing**; running a second cron recorder is optional and not required for
PNP4Nagios graphs.

## Nagios Core commands.cfg integration

Use the real absolute path of the binary installed on the monitored host:

```nagios
define command {
    command_name    check_whydown_memory
    command_line    /usr/local/bin/whydown nagios check --metric memory --warning $ARG1$ --critical $ARG2$
}
define command {
    command_name    check_whydown_disk
    command_line    /usr/local/bin/whydown nagios check --metric disk --path $ARG1$ --warning $ARG2$ --critical $ARG3$
}
define command {
    command_name    check_whydown_failed_units
    command_line    /usr/local/bin/whydown nagios check --metric failed-units
}
```

Then attach a service to your monitored host:

```nagios
define service {
    use                     generic-service
    host_name               linux-host
    service_description     WhyDown Memory
    check_command           check_whydown_memory!80!95
}
define service {
    use                     generic-service
    host_name               linux-host
    service_description     WhyDown Root Disk
    check_command           check_whydown_disk!/!85!95
}
```

Do not expose arbitrary `--path` arguments or monitoring command templates
to untrusted remote users. For a remote monitored host, run this plugin on
that host through your standard agent/check execution mechanism (NRPE,
NCPA, Icinga agent, SSH transport, etc.). WhyDown itself opens no listener.

If graphing within Nagios is required, configure the Core perfdata handling
and PNP4Nagios/RRD (or an equivalent collector), following your installed
version's documentation. The WhyDown output is already in the plugin's native
perfdata syntax; no custom parsing is required.

## Operational safety

- The monitoring check engine is separate from the full WhyDown
  `diagnose` command, avoiding full journal scans and expensive deep-disk
  diagnostics on each routine check.
- External `df` and `systemctl` probes are bounded by short timeouts.
  `--timeout` bounds the overall sampling deadline; status UNKNOWN is used
  when data cannot be collected.
- No privilege escalation or remote probing. A systemd manager or PSI
  interface missing on some hosts is UNKNOWN, not an invented zero.
- The CLI and desktop GUI continue working unchanged. Neither the automatic
  Nagios checks nor the offline charts require Python, Tkinter or Qt.
- Don't confuse the snapshot with a continuously running monitoring daemon.
  Scheduling, notifications, RRD retention and central graphs are provided
  by Nagios/Icinga and optional graphing backends, not by this mode.
