# Knowledge Graph Correctness Fixes - Changes Summary

## C-1: Cytoscape Initialization Fix

### Problem
Layout useEffect ran before cyRef.current was assigned, causing nodes to stay at origin (0,0) - the "single blob in corner" issue.

### Solution
Added cyReady state to ensure layout runs only after Cytoscape instance is fully initialized.

### Changes
1. Added cyReady state and ref
2. Modified cy callback to signal readiness and call cy.resize()
3. Updated event handler useEffect with cyReady dependency and cleanup
4. Updated layout useEffect to depend on cyReady with requestAnimationFrame scheduling

---

## C-2: Backend↔Frontend Doc Type Semantics Fix

### Problem
Backend encoded status into node_type string ("doc_deprecated") but frontend treated it as "kind". Created type confusion and incomplete filtering.

### Solution
Extended backend JSON with explicit `kind` and `status` fields. Updated frontend to use direct field access instead of string parsing.

### Backend Changes
1. Extended KgNodeJson struct with optional `kind`, `status`, and `path` fields
2. Updated knowledge_graph.rs to populate these fields from doc frontmatter
3. Keep node_type stable as "doc" for all document types

### Frontend Changes
1. Extended KnowledgeGraphNode type with optional kind/status/path fields
2. Updated filtering logic to use direct field access
3. Added proper status filtering (active/deprecated/archived)
4. Updated sidebar to show both kinds (code/decision/trajectory/preference) and statuses
5. Removed "deprecated" from kinds array - now handled via status filter
6. Focus mode now works for all doc nodes, not just "doc_*"

---

## Expected Behavior After Both Fixes
- Graph renders with nodes spread properly across screen
- 960+ nodes visible in overview (not just 1)
- Both kind and status filters work independently
- Deprecated docs filtered via status field
- No type parsing ambiguity
- Layout deterministic on mount
