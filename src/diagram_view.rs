use merman_core::{Engine, ParseOptions};
use merman_render::environment::RenderEnvironment;
use merman_render::{LayoutOptions, family};

/// Parse un texte mermaid, calcule son layout via merman (sans passer par le
/// rendu SVG) et log le JSON obtenu dans le terminal.
pub fn log_flowchart_layout(source: &str) {
    let engine = Engine::new();

    // NOTE: si `ParseOptions::default()` ne compile pas, vérifiez la vraie
    // définition de `ParseOptions` (cargo doc -p merman-core --open).
    let parsed = match engine.parse_diagram_for_render_model_sync(source, ParseOptions::default())
    {
        Ok(Some(parsed)) => parsed,
        Ok(None) => {
            tracing::error!("Le diagramme n'a pas pu être parsé");
            return;
        }
        Err(err) => {
            tracing::error!("Failed to parse diagram: {}", err);
            return;
        }
    };

    let env = RenderEnvironment::deterministic();
    let session = match env.begin_session() {
        Ok(session) => session,
        Err(err) => {
            tracing::error!("Failed to begin render session: {}", err);
            return;
        }
    };

    let artifact = match family::prepare(parsed, &LayoutOptions::default(), session) {
        Ok(artifact) => artifact,
        Err(err) => {
            tracing::error!("Failed to prepare layout: {}", err);
            return;
        }
    };

    match artifact.layout_json() {
        Ok(layout) => tracing::info!("Diagram layout JSON: {:#}", layout),
        Err(err) => tracing::error!("Failed to compute diagram layout: {}", err),
    }
}
