---
name: review-save-migration
description: Review an old Xianxia checkpoint reader or new persistent field and produce migration evidence.
---

Read contracts/README.md and the legacy reader for the affected version. Establish old-field defaults, regenerated indexes and accounting invariants. Run `python tools/dev.py test replay` and `python tools/dev.py test protocol`. Verify the migration using a copy of a fixture with the affected field absent. Record the source and resulting format, old-world defaults and both hashes. A different historical version needs its own representative input.
