# Development Process Specification

This document defines the 5-style documentation system used for developing R-DOS features.

## Philosophy

Traditional RFC-style development docs quickly become stale and disconnected from actual code. Instead, we use a **living documentation system** where different doc types serve different purposes and update at different cadences.

## The 5-Style System

### 1. Ultra/Plan Mode (Exploration & Design)

**Purpose**: Deep research and design before implementation
**Tools**: Claude Code's plan mode
**When**: At the start of every non-trivial feature

**Process**:
1. Enter plan mode (EnterPlanMode)
2. Launch Explore agents to understand existing code patterns
3. Ask clarifying questions to user via AskUserQuestion
4. Design implementation approach
5. Write comprehensive plan to plan file
6. Exit plan mode for user approval (ExitPlanMode)

**Artifacts**:
- Plan files in `~/.claude/plans/`
- Ephemeral - archived after feature completion
- Used only for current feature implementation

**Key Insight**: Pull in existing code and specs to inform design decisions. Don't design in a vacuum.

### 2. Beads Epics & Issues (Work Tracking)

**Purpose**: Track strategic work across sessions with dependencies
**Tools**: bd (beads CLI)
**When**: For multi-session work, features with dependencies, or discovered work

**Structure**:
```bash
bd create --title="Feature name" --type=feature --priority=2
bd create --title="Implementation task" --type=task --priority=2
bd dep add <task> <feature>  # Task depends on feature
```

**Lifecycle**:
- Created during/after planning
- Updated during implementation (in_progress, blocked, ready)
- Closed after user tests and pushes code
- Synced to git via `bd sync`

**Artifacts**:
- `.beads/issues.jsonl` (versioned in git)
- Persistent across sessions and compaction
- Searchable with `bd search`, `bd ready`, `bd blocked`

**Key Insight**: Issues capture "what needs doing" (ephemeral work), not "how it works" (that's specs).

### 3. Evergreen Specs (System Truth)

**Purpose**: Define how the system **should be** at all times
**Location**: `spec/` directory
**When**: Written after plan approval, updated when architecture changes

**Current Specs**:
- `spec/SPEC.md` - Overall feature specification
- `spec/PLUGIN.md` - Plugin development guide (MUST read before creating plugins)
- `spec/GAMES.md` - Games architecture and patterns
- `spec/OFFICE.md` - Office features specification
- `spec/ui.md` - ASCII layout reference (80x25 screen)
- `spec/strings/` - Authentic Q-DOS II messaging patterns

**Characteristics**:
- **Declarative**: "The system works like this" (not "we plan to make it work like this")
- **Architectural**: Focus on patterns, structures, and constraints
- **Stable**: Only updated when architecture actually changes
- **Reference material**: Agents read these during planning and implementation

**Example Content**:
```markdown
# Plugin Architecture

All plugins implement the Plugin trait with these methods:
- id() -> &str
- name() -> &str
- handle_global_key() - intercept keys before app
- handle_modal_key() - handle keys when modal is active
- draw_modal() - render plugin UI

Plugins MUST use FullScreenView for full-screen modals.
ModalFrame panics on areas larger than 79x23.
```

**Key Insight**: Specs document "what is" (evergreen truth), not "what was" (RFCs) or "what needs doing" (issues).

### 4. Skills (Agent Implementation Guides)

**Purpose**: Tell AI agents **how to implement** features following project patterns
**Location**: `.claude/skills/` directory
**When**: Created with initial feature, updated as patterns evolve

**Current Skills**:
- `rdos-rust-patterns` - Rust idioms for R-DOS
- `rdos-plugins` - How to implement plugins (code-level guidance)
- `rdos-ui-components` - How to use UI component library
- `rdos-content` - How to write authentic Q-DOS strings
- `rdos-games` - How to implement games
- `release` - How to release versions

**Characteristics**:
- **Prescriptive**: "Do it this way" (more specific than specs)
- **Code-focused**: Includes examples, patterns, anti-patterns
- **Agent-oriented**: Written for Claude Code consumption
- **Evolving**: Updated when new patterns emerge or best practices change

**Relationship to Specs**:
- **Specs say WHAT**: "Plugins use FullScreenView"
- **Skills say HOW**: "Create view like: `let view = FullScreenView::new(area, title, colors);`"

**Example Content**:
```markdown
## Creating a Plugin Modal

CRITICAL: Use FullScreenView for plugin modals, NOT ModalFrame!

```rust
use crate::ui::components::FullScreenView;

fn draw_modal(&self, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let view = FullScreenView::new(area, " Plugin Title ", colors);
    view.render_frame(frame);

    view.render_row(frame, 0, vec![Span::styled("Content", style)]);
    view.render_help(frame, vec![("Esc", "close")]);
}
```
```

**Key Insight**: Skills bridge the gap between "what the system is" (specs) and "write this code" (implementation).

### 5. User-Facing Docs (Human & Agent Communication)

**Purpose**: Explain the project to humans and end-user agents
**Location**: README files throughout repo
**When**: Updated with major features or API changes

**Current Docs**:
- `README.md` - Project overview, installation, usage
- `CLAUDE.md` - Guidance for Claude Code when working in this repo
- `crates/*/README.md` - Per-crate documentation
- Per-game docs (e.g., `src/plugins/games/ROGUE.md`)

**Characteristics**:
- **User-focused**: Written for humans reading on GitHub
- **Getting started**: Installation, basic usage, examples
- **Marketing**: Why use this, what problems it solves
- **Less technical**: Broader audience than specs/skills

**Key Insight**: READMEs are for humans browsing/using the repo. Specs and skills are for agents building features.

## Workflow: From Idea to Code

### Step 1: Plan Mode
```
User: "Add feature X"
Agent: <enters plan mode>
  1. Launches Explore agents to understand existing patterns
  2. Reads relevant specs (spec/SPEC.md, spec/PLUGIN.md, etc.)
  3. Asks user clarifying questions
  4. Designs implementation approach
  5. Writes plan to plan file
  6. Exits plan mode for approval
```

### Step 2: Beads Issues
```
Agent: <after plan approval>
  1. Creates beads epic for feature
  2. Breaks down into tasks/subtasks
  3. Sets up dependencies if needed
  bd create --title="Feature X" --type=feature
  bd create --title="Implement X logic" --type=task
  bd dep add <task> <feature>
```

### Step 3: Implementation
```
Agent: <during coding>
  1. Reads skills for "how to" guidance
  2. References specs for architectural decisions
  3. Updates beads issues (bd update <id> --status in_progress)
  4. Implements following patterns from specs + skills
  5. Writes code already formatted and linted
```

### Step 4: Update Specs & Skills
```
Agent: <if architecture changed>
  1. Updates relevant spec (e.g., spec/GAMES.md for new game pattern)
  2. Updates relevant skill (e.g., rdos-games for new implementation guide)
  3. Keeps spec declarative ("games work like this")
  4. Keeps skill prescriptive ("implement games like this")
```

### Step 5: Complete
```
Agent: <after testing>
  1. Commits code with git
  2. Syncs beads with bd sync
  3. DOES NOT close issues (user does after testing/pushing)
  4. DOES NOT push to remote (user does)
```

## Decision Framework

**When to create/update each doc type:**

| Situation | Action |
|-----------|--------|
| Starting new feature | Enter plan mode → write plan |
| Feature spans multiple sessions | Create beads issue |
| New architectural pattern | Update relevant spec in spec/ |
| New implementation pattern | Update relevant skill in .claude/skills/ |
| Public-facing change | Update README.md |
| Agent needs guidance | Update CLAUDE.md |

## Anti-Patterns

**DON'T:**
- ❌ Write RFC-style docs that get stale ("We plan to add X")
- ❌ Document temporary decisions in specs (use issues)
- ❌ Put implementation details in specs (use skills)
- ❌ Put architectural constraints in skills (use specs)
- ❌ Close beads issues before user tests and pushes
- ❌ Update docs after-the-fact (write correct code first time)

**DO:**
- ✅ Use plan mode for all non-trivial features
- ✅ Keep specs declarative and evergreen
- ✅ Keep skills prescriptive and code-focused
- ✅ Track multi-session work in beads
- ✅ Update specs/skills when patterns change
- ✅ Write formatted, linted code from the start

## Examples

### Example 1: Adding MINDGAMES

**Plan Mode**: Explored existing games (Brainiac, Clicker, DopeWars), designed state machine, wrote comprehensive plan

**Beads**: Created epic QDOS-tu77 with tasks for implementation, testing, integration

**Specs**: Will update `spec/GAMES.md` to document algorithmic content generation pattern

**Skills**: Will update `.claude/skills/rdos-games` to add MINDGAMES as reference implementation

**README**: No changes needed (internal feature)

### Example 2: Adding Plugin System

**Plan Mode**: Researched plugin architectures, designed Plugin trait, planned integration

**Beads**: Epic with tasks for trait, registration, lifecycle, rendering

**Specs**: Created `spec/PLUGIN.md` documenting Plugin trait, lifecycle, UI constraints

**Skills**: Created `.claude/skills/rdos-plugins` with step-by-step implementation guide

**README**: Updated with "Plugins" section and examples

## Benefits

1. **No stale docs**: Each doc type has a clear update trigger
2. **Agent-friendly**: Specs + skills provide complete context for AI implementation
3. **Human-friendly**: README provides getting-started without reading specs
4. **Searchable**: Beads issues track all work with dependencies
5. **Evergreen truth**: Specs document current architecture, not historical plans
6. **Efficient**: Plan mode prevents wasted implementation effort

## Maintenance

**Regular updates:**
- Specs: When architecture changes (rare)
- Skills: When patterns evolve (occasional)
- README: With major features or breaking changes (rare)
- CLAUDE.md: When agent guidance needs refinement (occasional)

**Per-feature updates:**
- Plan file: Created for each feature (then archived)
- Beads issues: Created/updated throughout feature lifecycle

**Never update:**
- Old plan files (ephemeral artifacts)
- Closed beads issues (historical record)

---

This system replaces traditional RFC-style development with a living documentation approach optimized for AI-assisted development.
