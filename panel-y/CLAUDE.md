# panel-y — Waveform Viewer App

**Concept**: A native Rust desktop app for interactively visualizing and analyzing waveform data. Python tools convert .mat / .csv inputs to Parquet. The older Dash/Plotly viewers are archived references.

## Directory Layout

```
factory/panel-y/
├── CLAUDE.md              ← this file
├── docs/                  ← design notes, planning docs, bug reports
│   ├── 企画書.md
│   ├── wave_viewer_roadmap.md
│   ├── proto_*_設計書.md
│   └── bug_report_*.md
├── journal/               ← development log (YYYY-MM-DD.md) ← the secretariat reads this
├── apps/panel-y/          ← active Rust viewer
├── tools/                 ← active Python preprocessing and fixture generators
├── data/                  ← shared samples and validation data
└── archive/               ← historical Python/Dash implementations
    ├── requirements.txt
    ├── proto_3_1/         ← earlier FFT/theme viewer and original tools
    └── proto_3_1b/        ← large-data viewer and backend benchmark
```

## Current State

For current development status and the next thing to work on, check the latest development log under `journal/`.
- Active implementation: `apps/panel-y/`; archived specification reference: `archive/proto_3_1b/`
- Roadmap: `docs/wave_viewer_roadmap.md`

## How to Develop

```bash
cd apps/panel-y
cargo run --release
```

For legacy Python comparisons, use the setup and launch commands in
[archive/README.md](archive/README.md). Current Python tools are documented in
[tools/README.md](tools/README.md).

## Coordination with the Secretariat

- **Development instructions do not come from the secretariat** — development is self-contained here
- **Progress sharing**: record in `journal/YYYY-MM-DD.md` → the secretariat transcribes it into the daily report
- **Unrelated to trading** — this is tool development for company work

## Code of Conduct

### The "When You're Rushed, Stop" Rule

"When you're rushed, stop" means "**step back and think from the basis of what the problem actually is**". When you hit a problem, don't jump to surface-level fixes — return to the problem-solving method:

1. **Grasp the current state**: organize what is happening as facts (separate facts from interpretation)
2. **Stratify the problem**: distinguish surface symptoms from the underlying problem (not "symptom → therefore X" but "symptom → what is the root cause → countermeasure against the root cause")
3. **Identify the root cause**: dig into why it happened
4. **Plan countermeasures**: think of moves that address the root cause

"When context runs low, take a break" is **one means** of securing this essential thinking. If clearing the context alone doesn't solve the problem, prioritize taking time to stop and organize your thoughts.
