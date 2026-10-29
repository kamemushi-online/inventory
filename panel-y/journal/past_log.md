# panel-y Past Dev Log

Extracted from the secretariat journal (`.secretary/journal/`). Transcribed 2026-03-27.

---
2026-03-10

- Kicked off a project to build a waveform-analysis viewer (per older notes)

---
2026-03-15

- Project kickoff, planning / design docs drafted (`/projects/panel_y`)
- Proto #1 practical viewer — implementation complete + committed (`d08a23b`)
  - Preprocessing utility (CSV/MAT→Parquet conversion)
  - Dash app (file selection + autocomplete, channel dropdown, measurement mode + cursor delta)
  - Functional check OK

---
2026-03-16

- Tidied up panel_y design documentation
  - proto_1_設計書.md: full update to match the implementation
  - 企画書.md: updated status, roadmap, and open questions
  - preprocessor_design.md: status → implemented, added multiconverter
- Created a new design / planning doc for Proto #1b (Panel + Bokeh edition)
  - Compared with the Dash version, sketched the Bokeh-specific implementation approach
  - Drafted a phased implementation plan
- Ran Serena onboarding

---
2026-03-22

- Moved panel-y from /newspace → /life/factory/panel-y; updated .gitignore and Serena config
- Ran panel-y onboarding (Serena memory + Explore agent)
- TIL: Serena's active project is determined by the working directory

---
2026-03-24

- Committed proto_0_3 on the feature/proto-0.3 branch
- Created Proto #0.4 (dropdown-UI style), functional check OK
- Drafted the waveform-display feature roadmap (`wave_viewer_roadmap.md`)
- Wrote design docs for Proto #0.3 and #0.4
- Finished Proto #2.0 (#1 + #0.4 integration)
  - Per-row dropdowns let you overlay multiple channels
  - Tried `make_subplots`, then pivoted to independent Graphs because hover-linking didn't work
  - Added scale lock (off by default; when unlocked, only the X axis zooms)
  - Added per-row numeric Y-axis range entry
  - SI-prefix notation on Y-axis ticks
  - Added the update keyboard shortcut (Ctrl+Enter)
  - Synced X-axis zoom across all rows (dropped the ctx.triggered_id branching in favor of always-apply-to-all)
- Switched to a shared venv (single .venv at panel-y root; removed per-proto venvs)
- Wrote Proto #2.0 design doc and a bug report
- Committed and pushed the feature/proto-2.0 branch
- TIL: Plotly's `make_subplots` has fundamental constraints — hover-linking and full-height spikelines don't work across subplots. Independent Graphs + Store synchronization is the only practical solution

---
2026-03-25

- Merged feature/proto-2.0 → main and pushed
- Implemented Phase 1-2 (line/step display toggle); functional check OK
- Implemented Phase 1-3 (line color / width selection); functional check OK
- UI improvements (wider + resizable Label column, better Value display, hidden legend, all-row hover fix)
- Implemented the cursor range analysis feature (Mean/Max/Min/P-P/RMS/Slope + Hz display)
- Proto #3.0 — Min-Max envelope decimation + zoom-linked display — implemented and pushed
- TIL: Min-Max envelope (keep min/max per bucket) is ideal for oscilloscope-style rendering. Compresses 100k → 2500 points without dropping peaks or spikes
- Carryover: Field-test Proto #3.0 against company data → tune the decimation threshold

---
2026-03-26

- Proto #3.1 design improvements (Dark/Light theme, 2-row card layout for channel settings, collapsible row definitions)
- Fixed a zoom-sync bug (zoom on the 2nd+ row didn't propagate to other rows)
- Centralized label-column-width adjustment in the header (ResizeObserver)
- Removed the duplicated Y-axis title
- Fixed a label-column-width-sync bug (per-element inline style → CSS variable approach + flex-shrink:0)
- Made the header bar sticky + added a 📌 pin toggle
- Committed and pushed Proto #3.1 (`31b0f54`)
- Serena: completed onboarding for the whole life repo + panel-y (5 memory entries created)
- TIL: Setting inline style.width on individual elements via ResizeObserver gets wiped when the DOM is regenerated during React re-renders. Setting a CSS custom property on the container is more robust
- Carryover: Field-test Proto #3.1 with company data → tune the decimation threshold; next candidates → signal processing (LPF), cursor-range FFT, or a command line
