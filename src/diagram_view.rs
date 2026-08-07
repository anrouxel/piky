use merman_core::{Engine, ParseOptions};
use merman_render::environment::RenderEnvironment;
use merman_render::{LayoutOptions, family};

/// Parse un texte mermaid, calcule son layout via merman (sans passer par le
/// rendu SVG) et log le JSON obtenu dans le terminal.
pub fn log_flowchart_layout(source: &str) {
    match layout_flowchart_json(source) {
        Ok(layout) => tracing::info!("Diagram layout JSON: {:#}", layout),
        Err(err) => tracing::error!("Failed to compute diagram layout: {}", err),
    }
}

fn layout_flowchart_json(source: &str) -> anyhow::Result<serde_json::Value> {
    let engine = Engine::new();

    // NOTE: si `ParseOptions::default()` ne compile pas, vérifiez la vraie
    // définition de `ParseOptions` (cargo doc -p merman-core --open).
    let parsed = engine
        .parse_diagram_for_render_model_sync(source, ParseOptions::default())?
        .ok_or_else(|| anyhow::anyhow!("le diagramme n'a pas pu être parsé"))?;

    let env = RenderEnvironment::deterministic();
    let session = env.begin_session()?;

    let artifact = family::prepare(parsed, &LayoutOptions::default(), session)?;

    Ok(artifact.layout_json()?)
}
