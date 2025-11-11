# Archived Documentation

This directory contains documentation for architectural paths **not taken** on the main branch.

## Context

During development, we explored two OAuth patterns:
1. **Authorization Server** (ADR-004) - Full OAuth proxy with DCR - **MAIN BRANCH**
2. **Resource Server** (ADR-005) - Claude handles OAuth - **SEPARATE WORKTREE/FORK**

The `feat/resource-server-pattern` worktree explores the simpler Resource Server pattern and may become a separate project.

## Archived Files

### ADR-005: Resource Server with Claude OAuth
- **Status**: Explored in separate worktree
- **Reason**: Main branch implements Authorization Server for full OAuth control
- **Location**: May become separate fork

### REFACTOR-BACKLOG.md
- **Status**: Backlog for Resource Server implementation
- **Reason**: Not applicable to Authorization Server path

### PIVOT-SUMMARY.md
- **Status**: Historical pivot analysis
- **Reason**: Decision made - keeping both paths in separate branches/projects
