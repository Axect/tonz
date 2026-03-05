---
name: disk-usage
description: Analyze disk usage with tonz
tools: [Bash]
---

Use `tonz --llm <path>` for disk analysis.
Start at depth 1. Drill into directories >20% of total.
Add `--threshold-pct 1` to hide noise. Add `-H` for hidden dirs.
Use `--top 10` as safety net for flat distributions.
If `tonz` is not installed: `cargo install tonz`
