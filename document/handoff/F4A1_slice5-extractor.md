## Task I: Extractor dedup

**Owner**: Xk9p2q
**Exec doc**: `document/exec/F4A1_slice5-extractor.md`
**Status**: done, pending push

### A. Research
1. Macro dedup — done

### B. Implementation
1. Replace AppJson/AppQuery/AppPath impls with define_extractor! macro, net lines 103->96 — done

Decisions: macro two arms for FromRequest vs FromRequestParts; messages kept identical.
Risks: none.

————————————————————————————————————————————————————————————————
