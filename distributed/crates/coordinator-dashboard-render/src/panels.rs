//! Flatten Grafana dashboard JSON into panels and datasource target kinds.

use serde_json::Value;

/// A single queryable dashboard panel.
#[derive(Debug, Clone)]
pub struct Panel {
    /// Grafana panel id.
    #[allow(dead_code)]
    pub id: u64,
    pub title: String,
    pub panel_type: String,
    pub targets: Vec<Value>,
}

impl Panel {
    pub fn is_row(&self) -> bool {
        self.panel_type == "row"
    }
}

/// Return non-row panels from top-level and collapsed-row panel arrays.
pub fn extract_panels(dashboard: &Value) -> Vec<Panel> {
    let mut out = Vec::new();
    let Some(panels) = dashboard.get("panels").and_then(Value::as_array) else {
        return out;
    };
    for panel in panels {
        walk(panel, &mut out);
    }
    out
}

fn walk(node: &Value, out: &mut Vec<Panel>) {
    let id = node.get("id").and_then(Value::as_u64).unwrap_or_default();
    let title = node.get("title").and_then(Value::as_str).unwrap_or_default().to_owned();
    let panel_type = node.get("type").and_then(Value::as_str).unwrap_or_default().to_owned();
    let targets = node.get("targets").and_then(Value::as_array).cloned().unwrap_or_default();

    let panel = Panel { id, title, panel_type, targets };
    if !panel.is_row() && !panel.title.is_empty() {
        out.push(panel);
    } else if let Some(children) = node.get("panels").and_then(Value::as_array) {
        for child in children {
            walk(child, out);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Prometheus,
    Infinity,
    Postgres,
    Unsupported,
}

impl TargetKind {
    pub fn from_target(target: &Value) -> Self {
        let ds_type = target
            .get("datasource")
            .and_then(|d| d.get("type"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        match ds_type {
            "prometheus" => Self::Prometheus,
            "yesoreyeram-infinity-datasource" => Self::Infinity,
            "grafana-postgresql-datasource" => Self::Postgres,
            _ => Self::Unsupported,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_walks_rows_and_skips_them() {
        let dashboard = json!({
            "panels": [
                {"id": 1, "type": "stat", "title": "Top", "targets": [{}]},
                {
                    "id": 2, "type": "row", "title": "Group",
                    "panels": [
                        {"id": 3, "type": "table", "title": "Inner", "targets": [{}, {}]}
                    ]
                }
            ]
        });
        let panels = extract_panels(&dashboard);
        let titles: Vec<&str> = panels.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(titles, vec!["Top", "Inner"]);
        assert_eq!(panels[0].targets.len(), 1);
        assert_eq!(panels[1].targets.len(), 2);
    }

    #[test]
    fn extract_skips_titleless_panels() {
        let dashboard = json!({
            "panels": [
                {"id": 1, "type": "stat", "title": "Visible", "targets": []},
                {"id": 2, "type": "stat", "targets": []}
            ]
        });
        let panels = extract_panels(&dashboard);
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].title, "Visible");
    }

    #[test]
    fn target_kind_classifies_by_datasource_type() {
        let prom = json!({"datasource": {"type": "prometheus", "uid": "prometheus"}});
        let inf = json!({"datasource": {"type": "yesoreyeram-infinity-datasource"}});
        let pg = json!({"datasource": {"type": "grafana-postgresql-datasource"}});
        let other = json!({"datasource": {"type": "loki"}});
        let none = json!({});
        assert_eq!(TargetKind::from_target(&prom), TargetKind::Prometheus);
        assert_eq!(TargetKind::from_target(&inf), TargetKind::Infinity);
        assert_eq!(TargetKind::from_target(&pg), TargetKind::Postgres);
        assert_eq!(TargetKind::from_target(&other), TargetKind::Unsupported);
        assert_eq!(TargetKind::from_target(&none), TargetKind::Unsupported);
    }
}
