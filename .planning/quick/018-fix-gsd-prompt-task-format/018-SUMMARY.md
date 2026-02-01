# Quick Task 018: Fix GSD Prompt Task Format

**Root cause of empty progress.md: format mismatch between GSD prompt and parser**

## Problem

The GSD plan prompt instructed Claude to output tasks in XML format:
```xml
<task>
  <name>Task name</name>
  <action>...</action>
  <verify>...</verify>
  <done>...</done>
</task>
```

But the ProgressFile parser only understands markdown checkbox format:
```markdown
- [ ] Task description
```

This caused the tasks array to be empty, triggering the new validation error from quick-017.

## Solution

Updated `prompts/gsd/PROMPT_plan.md` to use checkbox format with inline metadata:
```markdown
- [ ] Task description (verify: command, done: criteria)
```

Changes made:
1. Updated "Progress File Structure" section - replaced XML task examples with checkboxes
2. Updated "Task Structure" section - explained checkbox format instead of XML
3. Updated "Example Output" section - replaced all XML tasks with checkboxes
4. Updated "Notice how" comment - changed reference from XML to checkbox format

## Files Modified

- `prompts/gsd/PROMPT_plan.md` - Changed task format from XML to checkbox

## Verification

- All 110 E2E tests pass
- All prompts tests pass
- Clippy clean

## Commit

`faf6313` - fix(prompts): use checkbox task format in GSD plan prompt

---
*Quick task 018 completed: 2026-02-01*
