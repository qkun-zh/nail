export const meta = {"name":"reconstruct-enrich-map","description":"Draft the 2 feature PRD(s) of a reconstruction as a map-reduce (drafters return row proposals; the orchestrator is the single serial reducer)","phases":[{"title":"Draft"}]}

// NOT a plain Node script: launch via the Workflow tool — Workflow({ scriptPath: "/home/qkun/nail_new/document/reconstruction/orchestration/enrich-map.workflow.mjs" }).
// Emitted by `reconstruct --orchestrate` from the CURRENT worklist. The worklist is the source
// of truth: if it changes, re-run `--orchestrate --phase enrich-map` before launching.

// Constants for THIS reconstruction (injected at emit time; no Date.now/Math.random in this harness).
const OUT = "/home/qkun/nail_new/document/reconstruction"
const ENGINE = "/home/qkun/reconstruct-tool/scripts/analyze.mjs"
const WORKLIST = "/home/qkun/nail_new/document/reconstruction/inventory.json"
const AGENTS = OUT + '/orchestration/agents'
const BATCHES = [["01-project-setup"],["02-code"]]
const SCHEMA = {"type":"object","required":["proposals"],"properties":{"proposals":{"type":"array","items":{"type":"object","required":["slug","prd","interfaceRows","entityRows"],"properties":{"slug":{"type":"string"},"prd":{"type":"string","description":"the COMPLETE features/<slug>/PRD.md content — full spine, every callout resolved"},"interfaceRows":{"type":"array","description":"ROW PROPOSALS for architecture/INTERFACES.md (the orchestrator merges them)","items":{"type":"object","required":["method","path"],"properties":{"method":{"type":"string"},"path":{"type":"string"},"kind":{"type":"string"},"auth":{"type":"string"},"input":{"type":"string"},"output":{"type":"string"},"sideEffects":{"type":"array","items":{"type":"string"}}}}},"entityRows":{"type":"array","description":"ROW PROPOSALS for architecture/DATA-MODEL.md (the orchestrator merges them)","items":{"type":"object","required":["entity","fields"],"properties":{"entity":{"type":"string"},"fields":{"type":"array","items":{"type":"object","required":["name","type"],"properties":{"name":{"type":"string"},"type":{"type":"string"},"constraints":{"type":"string"},"enumRef":{"type":"string"}}}},"relations":{"type":"array","items":{"type":"string"}},"indexes":{"type":"array","items":{"type":"string"}},"uniques":{"type":"array","items":{"type":"string"}}}}},"enums":{"type":"array","description":"every enum the feature touches, with its COMPLETE member list","items":{"type":"object","required":["name","members"],"properties":{"name":{"type":"string"},"members":{"type":"array","items":{"type":"string"}},"description":{"type":"string"}}}},"notes":{"type":"string","description":"what the source could not settle (goes to unknowns, never into the PRD as fact)"}}}}}}

function contract(name, extra) {
  return 'Read and follow the dispatch contract at ' + AGENTS + '/' + name + '.md VERBATIM.\n'
    + 'Constants: OUT=' + OUT + '  ENGINE=' + ENGINE + '  WORKLIST=' + WORKLIST + '.\n'
    + 'Invoke the engine only by its ABSOLUTE path: node ' + ENGINE + ' <flags> — read-only flags only.'
    + (extra ? '\n' + extra : '')
}

log('reconstruct enrich-map: ' + "2" + ' item(s) across ' + BATCHES.length + ' agent(s)')

phase("Draft")
const results = await pipeline(BATCHES, (batch, _item, i) =>
  agent(contract('drafter', 'ITEMS=' + batch.join(',')), { label: 'enrich-map:' + (i + 1), phase: "Draft", agentType: 'general-purpose', schema: SCHEMA }))

// One-writer rule: this workflow only COLLECTS fragments. The main agent stays the single
// serial reducer — it folds them in itself. Next step:
//   merge the proposals into architecture/INTERFACES.md + architecture/DATA-MODEL.md and write each features/<slug>/PRD.md yourself (the serial REDUCE of references/orchestration.md), then gate: node /home/qkun/reconstruct-tool/scripts/analyze.mjs --check --out /home/qkun/nail_new/document/reconstruction
return { phase: "enrich-map", worklist: WORKLIST, results: results.filter(Boolean) }
