---
mode: "agent"
tools: ["changes", "runCommands"]
description: "Generate git branch name and commit message"
---

Generate git branch name and commit message using the following instructions:

1. Create a new **branch** if the current one doesn’t make sense:

   ```bash
   git checkout -b <branch-name>
   ```

2. Stage all changes:

   ```bash
   git add -A
   ```

3. A **git commit message** using the following command to commit the changes:

   ```bash
   git commit -m "<commit message>" --edit
   ```

Strictly follow the contributing [CONTRIBUTING.md](../../CONTRIBUTING.md) guide.
