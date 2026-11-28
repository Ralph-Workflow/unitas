use super::compat_finding::sanitize_content_identity;
use super::{CompatibilityState, ContentIdentity, Finding, Tier};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderGraphTarget {
    BuiltIn,
    Universal,
    HighDefinition,
    VisualEffect,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ShaderGraphNodeIssue {
    pub node_type: String,
    pub node_id: String,
}

fn shader_graph_label(content: &ContentIdentity) -> String {
    content
        .asset_path
        .clone()
        .or_else(|| content.object_name.clone())
        .or_else(|| content.guid.clone())
        .unwrap_or_else(|| "ShaderGraph".to_string())
}

fn affected_materials_clause(affected_materials: &[String]) -> String {
    if affected_materials.is_empty() {
        "no affected materials".to_string()
    } else {
        format!("affected materials: {}", affected_materials.join(", "))
    }
}

fn hdrp_shader_graph_finding(
    content: ContentIdentity,
    graph_label: &str,
    materials_clause: &str,
) -> Finding {
    Finding {
        feature_id: "REN-006".to_string(),
        tier: Tier::Native,
        state: CompatibilityState::Unsupported,
        content,
        behavior: format!("Shader Graph '{graph_label}' with HDRP graph target"),
        relied_upon_unity_differences: Vec::new(),
        observable_consequence: format!(
            "Shader Graph HDRP graph target is Unsupported under REN-006; {materials_clause}"
        ),
        conditions: vec!["Shader Graph target is HDRP".to_string()],
        related_findings: Vec::new(),
    }
}

fn visual_effect_shader_graph_finding(
    content: ContentIdentity,
    graph_label: &str,
    materials_clause: &str,
) -> Finding {
    Finding {
        feature_id: "REN-006".to_string(),
        tier: Tier::Native,
        state: CompatibilityState::Unsupported,
        content,
        behavior: format!("Shader Graph '{graph_label}' with Visual Effect Graph target"),
        relied_upon_unity_differences: Vec::new(),
        observable_consequence: format!(
            "Shader Graph Visual Effect Graph target is Unsupported under REN-006; {materials_clause}"
        ),
        conditions: vec!["Shader Graph target is Visual Effect Graph".to_string()],
        related_findings: vec!["REN-004".to_string()],
    }
}

fn unrepresentable_shader_graph_finding(
    content: ContentIdentity,
    graph_label: &str,
    materials_clause: &str,
    unrepresentable_nodes: &[ShaderGraphNodeIssue],
) -> Finding {
    let nodes_summary: Vec<String> = unrepresentable_nodes
        .iter()
        .map(|issue| format!("{} ({})", issue.node_type, issue.node_id))
        .collect();
    Finding {
        feature_id: "REN-006".to_string(),
        tier: Tier::Native,
        state: CompatibilityState::Unsupported,
        content,
        behavior: format!(
            "Shader Graph '{graph_label}' uses unrepresentable node(s): {}",
            nodes_summary.join(", ")
        ),
        relied_upon_unity_differences: Vec::new(),
        observable_consequence: format!(
            "graph '{graph_label}' cannot be represented due to node(s) {} and renders as a failed shader; {materials_clause}",
            nodes_summary.join(", ")
        ),
        conditions: vec!["Shader Graph contains unrepresentable node".to_string()],
        related_findings: Vec::new(),
    }
}

fn supported_shader_graph_finding(
    content: ContentIdentity,
    graph_label: &str,
    materials_clause: &str,
) -> Finding {
    Finding {
        feature_id: "REN-006".to_string(),
        tier: Tier::Native,
        state: CompatibilityState::Supported,
        content,
        behavior: format!("Shader Graph '{graph_label}'"),
        relied_upon_unity_differences: Vec::new(),
        observable_consequence: format!(
            "Shader Graph '{graph_label}' imports and translates cleanly; {materials_clause}"
        ),
        conditions: vec![
            "Shader Graph target is URP or Built-in with representable nodes".to_string(),
        ],
        related_findings: Vec::new(),
    }
}

pub fn assess_shader_graph(
    graph_content: ContentIdentity,
    target: ShaderGraphTarget,
    unrepresentable_nodes: &[ShaderGraphNodeIssue],
    affected_materials: &[String],
) -> Finding {
    let mut content = sanitize_content_identity(&graph_content);
    let graph_label = shader_graph_label(&content);
    let materials_clause = affected_materials_clause(affected_materials);

    match target {
        ShaderGraphTarget::HighDefinition => {
            hdrp_shader_graph_finding(content, &graph_label, &materials_clause)
        }
        ShaderGraphTarget::VisualEffect => {
            visual_effect_shader_graph_finding(content, &graph_label, &materials_clause)
        }
        ShaderGraphTarget::BuiltIn | ShaderGraphTarget::Universal => {
            if let Some(first_issue) = unrepresentable_nodes.first() {
                if content.sub_asset_id.is_none() {
                    content.sub_asset_id = Some(first_issue.node_id.clone());
                }
                unrepresentable_shader_graph_finding(
                    content,
                    &graph_label,
                    &materials_clause,
                    unrepresentable_nodes,
                )
            } else {
                supported_shader_graph_finding(content, &graph_label, &materials_clause)
            }
        }
    }
}
