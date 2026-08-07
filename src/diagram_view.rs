use merman_core::diagram::RenderSemanticModel;
use merman_core::{Engine, ParseOptions};
use merman_render::LayoutOptions;

/// Parse un texte mermaid et log, pour chaque nœud, son id/label/forme
/// (`layoutShape`, ex: "squareRect", "diamond", "stadium"...).
///
/// Cette info vient du modèle sémantique (public), pas du layout: le
/// layout ne contient que la géométrie (x/y/width/height), pas la forme.
pub fn log_flowchart_shapes(source: &str) {
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

    let RenderSemanticModel::Flowchart(model) = &parsed.model else {
        tracing::error!("Ce diagramme n'est pas un flowchart");
        return;
    };

    for node in &model.nodes {
        tracing::info!(
            "node id={:?} label={:?} shape={:?}",
            node.id,
            node.label,
            node.layout_shape
        );
    }
}

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
