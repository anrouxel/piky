//! `classDiagram` support: parses via merman-core, lays out via
//! merman-render (`merman_render::class`), and draws with GSK.
//!
//! Split by element kind for maintainability — add a new file here (plus a
//! dispatch arm in `scene::ClassDiagramScene::draw`/`hit_test`) when a new
//! kind of class-diagram element needs its own drawing logic.

mod cluster;
mod edge;
mod node;
mod note;
mod scene;

pub use scene::ClassDiagramScene;
