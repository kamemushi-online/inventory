---
name: "source-command-end-session"
description: "End-of-session routine (memory update + journal entry + commit + push check)"
---

# source-command-end-session

Use this skill when the user asks to run the migrated source command `end-session`.

## Command Template

# Session-End Procedure

Record this session as a checkpoint. Run the following 4 steps in order.

> Standard spec: `life/.Codex/rules/factory-journal.md`
> Source: deployed for panel-y from the factory shared template (`life/templates/factory/.Codex/commands/end-session.md`)

## Preconditions

- Check the current `git status`. **If there are uncommitted work changes (changes other than journal/memory), commit those first** before moving on to the journal step.
- If there is no uncommitted work, just proceed.

## Step 1: Update project memory (when applicable)

**Note: panel-y currently has no project memory (skip)**

(For reference: if memory is introduced in the future, the target will be `~/.Codex/projects/-Users-sugusokothx-life-factory-panel-y/memory/project_status.md`. Enable this section when it is introduced.)

- Append this session's progress to the end of the existing "next steps" section.
- The appended content should include:
  - Main tasks completed in this session (with commit hashes)
  - Important design decisions and disciplines that were finalized
  - The starting point for the next session (what to do next, prerequisite knowledge, things to watch out for)
- If the new content overlaps with what is already written, update the existing section instead of appending new text.
- Other sections such as "## Things to Watch Out For" should also be updated if any part has become stale due to today's work.

## Step 2: Create the journal

Target file: `journal/YYYY-MM-DD.md` (today's date)

- **Append to the existing file if one exists**; create a new one if not.
- Format standard: see `life/.Codex/rules/factory-journal.md`

```markdown
# YYYY-MM-DD Development Log

## Today's Theme
(1–2 paragraphs describing the main goal of the session)

## Approach Agreement
(How the decisions came about, the option chosen vs. the options rejected)

## Deliverables
(Commits / new files / design docs, etc.)

## Key Discoveries Today
(Technical insights, design decisions, operational rules)

## Review (only when applicable)
(Gemini/GPT/user feedback + how it was incorporated)

## Tomorrow Onward
(Starting point for the next session)

## Reflections
(Retrospective on the session, meta-level observations)
```

Pick and choose sections according to the content.

## Step 3: Commit the journal as one commit

- Add only `journal/YYYY-MM-DD.md`.
- Commit message format: `docs(journal): YYYY-MM-DD 開発日誌 — <main theme>`.
- Bullet today's main topics in the body.
- Per existing convention, include `Co-Authored-By: Codex Opus 4.7 (1M context) <noreply@anthropic.com>`.

## Step 4: Confirm push

- **Do not auto-push**. Always ask the user "Should I push?".
- Show how many commits the local branch is ahead by (`git status` output, or `git log origin/main..HEAD --oneline`).
- Run `git push origin main` only after the user says OK.

## Closing Report

After the push completes (or is deferred), report the following in a single message:
- List of commits made in this session (short hash + one-line summary)
- The starting point for the next session (consistent with what was written to memory)
- If needed, a reminder suggestion for long-term TODOs (a natural suggestion of `/schedule`, not mandatory)
