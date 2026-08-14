# Project skeleton (fixed, must not be changed)

Appendix to README §3. The skeleton fixes only the top-level entry files per
layer; the module trees beneath the backend and frontend layers are not
prescribed here and must be designed fresh — never copied from the legacy
`nail` layouts (README §4.1, §4.2).

```text
nail_new/
|-- README.md
|-- code/
|   |-- back/
|   |   |-- Cargo.toml
|   |   `-- src/
|   |       |-- main.rs
|   |       |-- interface.rs
|   |       |-- interface/
|   |       |-- logic.rs
|   |       |-- logic/
|   |       |-- repository.rs
|   |       |-- repository/
|   |       |-- infrastructure.rs
|   |       `-- infrastructure/
|   |-- common/
|   |   |-- Cargo.toml
|   |   `-- src/
|   |       |-- lib.rs
|   |       |-- text.rs
|   |       |-- text/
|   |       |-- name.rs
|   |       |-- name/
|   |       |-- tag.rs
|   |       |-- tag/
|   |       |-- response.rs
|   |       |-- response/
|   |       |-- hash.rs
|   |       |-- hash/
|   |       |-- time.rs
|   |       |-- time/
|   |       |-- pow.rs
|   |       |-- pow/
|   |       |-- request.rs
|   |       |-- request/
|   |       |-- search.rs
|   |       `-- search/
|   `-- front/
|       |-- Cargo.toml
|       `-- src/
|           |-- main.rs
|           |-- page.rs
|           |-- page/
|           |-- request.rs
|           |-- request/
|           |-- router.rs
|           |-- router/
|           |-- infrastructure.rs
|           `-- infrastructure/
|-- configuration/
|-- data/
|-- log/
|-- document/
`-- test/
```

> The `common` module list was settled in Phase 2 — `text`, `name`, `tag`,
> `response`, `hash`, `time`, `pow`, `request`, `search`, each a same-named
> `.rs` + folder pair (§4.4).
