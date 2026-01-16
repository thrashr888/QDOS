---
name: development
description: R-DOS development process. Use when planning features, creating documentation, tracking work with beads, or understanding the documentation system.
allowed-tools: Read, Edit, Write, Bash, Grep, Glob, Task, EnterPlanMode, ExitPlanMode, AskUserQuestion, TodoWrite
---

# R-DOS Development Process Skill

Use this skill when planning features, creating documentation, tracking work with beads,
or understanding how the 5-style documentation system works.

## Quick Reference

**Spec**: `spec/DEVELOPMENT.md` - Full process specification
**Issues**: `.beads/` - Git-tracked issue database
**Plans**: `plans/` - Preserved plan history (git-tracked)

## The 5-Style Documentation System

R-DOS uses 5 distinct doc types, each with a specific purpose:

| Style              | Purpose                           | Location          | Update Trigger       |
| ------------------ | --------------------------------- | ----------------- | -------------------- |
| 1. Plan Mode       | Deep exploration before coding    | `plans/`          | Per feature          |
| 2. Beads Issues    | Track work across sessions        | `.beads/`         | Throughout feature   |
| 3. Evergreen Specs | System truth ("how it works")     | `spec/`           | Architecture changes |
| 4. Skills          | Agent guides ("how to implement") | `.claude/skills/` | Pattern changes      |
| 5. User Docs       | Human communication               | README, CLAUDE.md | Major features       |

## Workflow: From Idea to Code

### Step 1: Enter Plan Mode

For any non-trivial feature:

```
1. Use EnterPlanMode tool
2. Launch Explore agents to understand existing patterns
3. Read relevant specs (spec/SPEC.md, spec/PLUGIN.md, etc.)
4. Ask clarifying questions via AskUserQuestion
5. Design implementation approach
6. Write plan to plan file
7. Exit plan mode with ExitPlanMode for approval
8. Copy approved plan to plans/ directory (see naming below)
```

**Plan file naming**: `plans/YYYY-MM-DD-feature-name.md`

Example: `plans/2026-01-15-external-plugins.md`

Plans are preserved even if abandoned - they document design decisions and exploration history.

### Step 2: Create Beads Issues

After plan approval:

```bash
# Create feature epic
bd create --title="Feature X" --type=feature --priority=2

# Break into tasks
bd create --title="Implement X logic" --type=task --priority=2
bd create --title="Add X tests" --type=task --priority=2

# Set dependencies
bd dep add <task-id> <feature-id>  # Task depends on feature

# Claim work
bd update <id> --status=in_progress
```

### Step 3: Implementation

During coding:

```
1. Read skills for "how to" guidance
2. Reference specs for architecture
3. Update beads (bd update <id> --status=in_progress)
4. Write code already formatted and linted (cargo fmt, cargo clippy)
5. Follow patterns from specs + skills
6. Run tests frequently (cargo test)
```

### Step 4: Update Specs & Skills (if architecture changed)

```
- Specs: Declarative ("plugins work like this")
- Skills: Prescriptive ("implement plugins like this")
```

### Step 5: Complete

```bash
# Verify quality gates
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test

# Commit code
git add .
git commit -m "Implement Feature X"

# Sync beads
bd sync

# DO NOT close issues (user does after testing/pushing)
# DO NOT push to remote (user does)
```

## Decision Framework

| Situation                  | Action                            |
| -------------------------- | --------------------------------- |
| Starting new feature       | Enter plan mode                   |
| Feature spans sessions     | Create beads issue                |
| New architectural pattern  | Update spec in `spec/`            |
| New implementation pattern | Update skill in `.claude/skills/` |
| Public-facing change       | Update README.md                  |
| Agent guidance needed      | Update CLAUDE.md                  |

## Plan Preservation

Plans are git-tracked in `plans/` to preserve institutional memory:

- **Approved plans** - Document the "why" behind implementations
- **Abandoned plans** - Show alternatives considered and why they were rejected
- **Partial plans** - Capture exploration that may be revisited later

Plans complement beads issues: beads track _what_ work was done, plans track _how_ decisions were made.

**When to save a plan:**

- After plan approval (before implementation)
- When abandoning a plan (add note explaining why)
- When significantly revising approach mid-implementation

## Specs vs Skills

**Specs say WHAT** (evergreen truth):

```markdown
# Plugin Architecture

All plugins implement the Plugin trait.
Plugins MUST use FullScreenView for full-screen modals.
```

**Skills say HOW** (implementation guide):

````markdown
## Creating a Plugin Modal

```rust
let view = FullScreenView::new(area, " Title ", colors);
view.render_frame(frame);
view.render_help(frame, vec![("Esc", "close")]);
```
````

## Beads Essentials

### Finding Work

```bash
bd ready               # Show issues ready to work (no blockers)
bd list --status=open  # All open issues
bd blocked             # Show blocked issues
bd show <id>           # View issue details
```

### Managing Work

```bash
bd update <id> --status=in_progress  # Claim work
bd close <id>                        # Mark complete (ONLY after user tests)
bd sync                              # Sync with git remote
```

### Dependencies

```bash
bd dep add <issue> <depends-on>  # Add dependency
```

## Quality Gates

R-DOS requires ALL THREE to pass before commits:

```bash
cargo fmt -- --check       # Code formatting
cargo clippy -- -D warnings  # Linting
cargo test                 # Tests
```

Run all three with:

```bash
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```

## Anti-Patterns

**DON'T:**

- Write RFC-style docs that get stale ("We plan to add X")
- Document temporary decisions in specs (use issues)
- Put implementation details in specs (use skills)
- Put architectural constraints in skills (use specs)
- Close beads issues before user tests and pushes
- Push to remote without user approval
- Skip quality gates (fmt, clippy, test)

**DO:**

- Use plan mode for all non-trivial features
- Save plans to `plans/` directory after approval (or abandonment)
- Keep specs declarative and evergreen
- Keep skills prescriptive and code-focused
- Track multi-session work in beads
- Update specs/skills when patterns change
- Write formatted, linted code from the start
- Run `cargo test` frequently during development

## Issue Lifecycle Rules

**CRITICAL: Follow this order:**

1. **Code complete** - Implementation is done, tests pass
2. **Commit code** - Create git commit(s)
3. **Wait for user** - DO NOT close issues yet
4. **User tests** - Let user test the implementation
5. **User pushes** - User pushes code to remote
6. **Then close issues** - Only close after code is pushed and tested

**Epics** should only be closed after:

- All child issues are closed
- The feature has been tested by user
- The code has been pushed to remote

## Key Specs Reference

| Spec               | Purpose                                        |
| ------------------ | ---------------------------------------------- |
| `spec/SPEC.md`     | Overall feature specification                  |
| `spec/PLUGIN.md`   | Plugin development guide (MUST read for plugins) |
| `spec/GAMES.md`    | Games architecture and patterns                |
| `spec/OFFICE.md`   | Office features specification                  |
| `spec/ui.md`       | ASCII layout reference (80x25 screen)          |
| `spec/strings/`    | Authentic Q-DOS II messaging patterns          |
