  # Feature delta between #0.2 and #1                                                                                
                                                                                                                      
  Features carried over from #0.2                                                                                     

  ┌─────┬──────────────────────────────────────────────────────────────┐
  │  #  │                           Feature                            │
  ├─────┼──────────────────────────────────────────────────────────────┤
  │ 1   │ Row layout [ch name/value] [waveform graph]                  │
  ├─────┼──────────────────────────────────────────────────────────────┤
  │ 2   │ Parquet data loading (path is hardcoded)                     │
  ├─────┼──────────────────────────────────────────────────────────────┤
  │ 3   │ Automatic channel detection / multi-channel display          │
  ├─────┼──────────────────────────────────────────────────────────────┤
  │ 4   │ Zoom/pan synced across all graphs                            │
  ├─────┼──────────────────────────────────────────────────────────────┤
  │ 5   │ All-channel hover values (left column) + cross-graph spike    │
  ├─────┼──────────────────────────────────────────────────────────────┤
  │ 6   │ Scroll zoom / toolbar (shown on hover)                       │
  └─────┴──────────────────────────────────────────────────────────────┘

  ---
  Additions and improvements in #1

  🆕 New (not present in #0.2)

  ┌─────┬─────────────────────────────────────────────────────┬────────────┐
  │  #  │                       Feature                       │  Category  │
  ├─────┼─────────────────────────────────────────────────────┼────────────┤
  │ A   │ File-selection UI (load any Parquet file)           │ Data input │
  ├─────┼─────────────────────────────────────────────────────┼────────────┤
  │ B   │ Data preprocessing utility (CSV/MAT → Parquet)      │ Data input │
  ├─────┼─────────────────────────────────────────────────────┼────────────┤
  │ C   │ Channel show/hide toggle                            │ UI         │
  ├─────┼─────────────────────────────────────────────────────┼────────────┤
  │ D   │ Cursor difference calculation (Δt / ΔV / ΔA)        │ Analysis   │
  └─────┴─────────────────────────────────────────────────────┴────────────┘

  🔧 Known issues from #0.2 (from §4 "Improvements" in the spec)

  ┌─────┬───────────────────────────────────────────┬──────────────────────────────────────────┐
  │  #  │                   Issue                   │                 Approach                 │
  ├─────┼───────────────────────────────────────────┼──────────────────────────────────────────┤
  │ E   │ Y-axis unit not shown (channel name only) │ Attach unit via metadata JSON or column name convention │
  ├─────┼───────────────────────────────────────────┼──────────────────────────────────────────┤
  │ F   │ Hover value does not reset to --- on mouse-out │ Reset the Store on the mouseleave event │
  └─────┴───────────────────────────────────────────┴──────────────────────────────────────────┘

  ▎ G "Show hover value inside the graph" and H "UI design for cursor diff (marker vs. range-selector)"
  are still listed as undecided in the planning document, so the #1 design needs to settle them.

  ---
  Summary: how #0.2 → #1 will change

  #0.2 (done)                       #1 (additions / improvements)
  ─────────────────────────────────────────────────
  Hardcoded path             →  File-selection UI (A)
  (no preprocessing)         →  CSV/MAT→Parquet conversion in utils/ (B)
  All channels always shown  →  Channel show/hide toggle (C)
  (no diff calculation)      →  Δt/ΔV/ΔA cursor diff (D)
  Y-axis without unit        →  Y-axis with unit (E)
  Value persists on mouse-out →  Mouse-out resets to --- (F)

  ---
  Next-action candidates:
  - Write the design doc → take the above as the base of proto_1_設計書.md
  - Start from implementation → A (file-selection UI) is the highest priority (removes hardcoding), so start there
