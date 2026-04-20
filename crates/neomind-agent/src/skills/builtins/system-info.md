---
id: system-info
name: System Information Query
category: agent
origin: builtin
priority: 60
token_budget: 600
triggers:
  keywords: [system info, system time, device info, system language, os version, platform info, hostname, cpu info, memory usage, disk usage, uptime, system status, what time, current time, get time, system details, environment info]
---

# System Information Query

Use the `shell` tool to gather system information. Execute the appropriate commands below based on the user's request.

## Get Current Date and Time

```bash
date
```

For ISO 8601 format:

```bash
date -u +"%Y-%m-%dT%H:%M:%SZ"
```

## Get OS and Platform Information

### Linux
```bash
cat /etc/os-release 2>/dev/null || cat /etc/issue 2>/dev/null
uname -a
```

### macOS
```bash
sw_vers
uname -a
```

### Cross-platform (Rust/Tauri environment)
```bash
uname -s -m -r
```

## Get Hostname

```bash
hostname
```

## Get System Language and Locale

```bash
echo "LANG=$LANG"
echo "LC_ALL=$LC_ALL"
locale 2>/dev/null || true
```

## Get CPU Information

### Linux
```bash
lscpu 2>/dev/null || cat /proc/cpuinfo | head -20
```

### macOS
```bash
sysctl -n machdep.cpu.brand_string 2>/dev/null
sysctl -n hw.ncpu 2>/dev/null
```

## Get Memory Usage

### Linux
```bash
free -h 2>/dev/null || cat /proc/meminfo | head -5
```

### macOS
```bash
vm_stat | head -10
sysctl -n hw.memsize 2>/dev/null
```

## Get Disk Usage

```bash
df -h / 2>/dev/null || df -h
```

## Get System Uptime

```bash
uptime
```

## Get Network Information

```bash
hostname -I 2>/dev/null || ifconfig 2>/dev/null | grep "inet " | head -5
```

## Combined Quick Overview

For a quick system overview, combine commands:

```bash
echo "=== System Overview ===" && \
echo "Hostname: $(hostname)" && \
echo "Time: $(date)" && \
echo "Uptime: $(uptime)" && \
echo "OS: $(uname -s) $(uname -r)" && \
echo "Arch: $(uname -m)" && \
echo "Locale: $LANG"
```

## Important: OS Differences

Commands vary significantly across operating systems. ALWAYS detect the OS first before choosing which commands to run:

```bash
uname -s
```

- **Linux**: Use `free`, `lscpu`, `cat /proc/*`, `/etc/os-release`
- **macOS**: Use `sysctl`, `vm_stat`, `sw_vers`
- **Windows** (WSL/Git Bash): Use `wmic`, `systeminfo`
- **Cross-platform safe**: `date`, `hostname`, `uname`, `df`, `uptime`, `echo $LANG`

When unsure, use fallback chains: `command1 2>/dev/null || command2 2>/dev/null || echo "not available"`

## Notes

- Always use the `shell` tool to run these commands: `shell(command="date")`
- On Tauri desktop (NeoMind Edge), the shell tool runs commands on the host machine
- Detect the OS first with `uname -s` to pick the right commands
- Combine multiple small commands into one shell call when possible to reduce round trips
