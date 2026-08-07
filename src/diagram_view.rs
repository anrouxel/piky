use merman_core::{Engine, ParseOptions};
use merman_render::LayoutOptions;

/// Parse un texte mermaid, calcule son layout via merman (sans passer par le
/// rendu SVG) et log le JSON obtenu dans le terminal.
pub fn log_flowchart_layout(source: &str) {
    let engine = Engine::new();

    let parsed = match engine.parse_diagram_for_render_model_sync(source, ParseOptions::lenient())
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

    let layout =
        match merman_render::layout_parsed_render_layout_only(&parsed, &LayoutOptions::default())
        {
            Ok(layout) => layout,
            Err(err) => {
                tracing::error!("Failed to compute diagram layout: {}", err);
                return;
            }
        };

    match serde_json::to_value(&layout) {
        Ok(json) => tracing::info!("Diagram layout JSON: {:#}", json),
        Err(err) => tracing::error!("Failed to serialize diagram layout: {}", err),
    }
}
