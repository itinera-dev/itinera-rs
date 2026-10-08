@AGENTS.md

## Claude Code

Everything above is [AGENTS.md](AGENTS.md), the manual shared with every contributor and every other agent. Claude Code does not read `AGENTS.md` on its own, so this file imports it. The import must stay at the top.

Below is only what is specific to Claude Code.

- **The `review-change` skill** in `.claude/skills/review-change/` is how Claude has a change reviewed by a separate reviewer agent, with the checklist that reviewer follows.
- **The `stack` skill** in `.claude/skills/stack/` is how Claude opens, links and follows a stack of pull requests, and why upper layers are not rebased by hand.
