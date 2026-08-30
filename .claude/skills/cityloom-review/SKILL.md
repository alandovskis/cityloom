---
name: cityloom-review
description: >
  Use when the task asks for code review, before completing tasks with
  human_review: true, or when the accumulated diff exceeds 300 lines.
---

# Shiplog review

## Process
1. Run scripts/review-context.sh — it assembles the review package
   (branch diff + touched features + the task's acceptance). Do NOT
   assemble review context by hand; the script exists for that.
2. Review AGAINST the acceptance and the features, in that order. Style
   is the linter's job, not yours.
3. Classify each finding: BLOCKS (violates acceptance/feature/a skill
   non-negotiable) or SUGGESTION (everything else). Only BLOCKS prevents
   completion.
