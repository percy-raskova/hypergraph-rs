//! BIGG metabolic-network parsing (XGI `readwrite/bigg_data.py`).
//!
//! Offline dict → DiHypergraph. The HTTP fetch (`load_bigg_data`) is behind the
//! `network` feature and unimplemented (see the plan).

use serde_json::Value;

use super::ReadWriteError;
use crate::DiHypergraph;

/// Convert a BIGG model dict (optionally with the index dict for organism metadata)
/// into a DiHypergraph. XGI `_bigg_to_dihypergraph`.
pub fn bigg_dict_to_dihypergraph(
    model: &Value,
    index: Option<&Value>,
) -> Result<DiHypergraph, ReadWriteError> {
    let mut dh = DiHypergraph::new();

    let id = model
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ReadWriteError::Format("BIGG model missing id".into()))?;
    dh.set_graph_attr("name", Value::String(id.to_string()));

    if let Some(idx) = index {
        if let Some(results) = idx.get("results").and_then(|v| v.as_array()) {
            if let Some(info) = results
                .iter()
                .find(|it| it.get("bigg_id").and_then(|v| v.as_str()) == Some(id))
            {
                if let Some(org) = info.get("organism") {
                    dh.set_graph_attr("organism", org.clone());
                }
            }
        }
    }

    // metabolites -> nodes
    let metabolites = model
        .get("metabolites")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ReadWriteError::Format("BIGG model missing metabolites".into()))?;
    for m in metabolites {
        if let Some(mid) = m.get("id").and_then(|v| v.as_str()) {
            let name = m.get("name").cloned().unwrap_or(Value::Null);
            dh.add_node(mid, serde_json::json!({ "name": name }));
        }
    }

    // reactions -> directed edges
    let reactions = model
        .get("reactions")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ReadWriteError::Format("BIGG model missing reactions".into()))?;
    for r in reactions {
        let rid = r.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let l = r.get("lower_bound").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let u = r.get("upper_bound").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let coeffs = match r.get("metabolites").and_then(|v| v.as_object()) {
            Some(o) => o,
            None => continue,
        };

        let split = |positive_is_product: bool| -> (Vec<String>, Vec<String>) {
            let mut reactants = Vec::new();
            let mut products = Vec::new();
            for (met, val) in coeffs {
                let v = val.as_f64().unwrap_or(0.0);
                let is_product = if positive_is_product {
                    v > 0.0
                } else {
                    v >= 0.0
                };
                if is_product {
                    products.push(met.clone());
                } else {
                    reactants.push(met.clone());
                }
            }
            (reactants, products)
        };

        // forward: l>=0 && u>0
        if l >= 0.0 && u > 0.0 {
            let (reactants, products) = split(true);
            if !(reactants.is_empty() && products.is_empty()) {
                let _ = dh.add_edge(reactants, products, Some(rid.to_string()), Value::Null);
            }
        }
        // reverse: l<0 && u<=0  (positive coeff -> reactant)
        if l < 0.0 && u <= 0.0 {
            let (products, reactants) = {
                // positive_is_product=false flips roles per XGI's reverse branch
                let mut reactants = Vec::new();
                let mut products = Vec::new();
                for (met, val) in coeffs {
                    let v = val.as_f64().unwrap_or(0.0);
                    if v >= 0.0 {
                        reactants.push(met.clone());
                    } else {
                        products.push(met.clone());
                    }
                }
                (products, reactants)
            };
            if !(reactants.is_empty() && products.is_empty()) {
                let _ = dh.add_edge(reactants, products, Some(rid.to_string()), Value::Null);
            }
        }
        // reversible: l<0 && u>0 -> forward + <id>_reverse
        // XGI's reversible branch classifies only val>0 -> product, val<0 ->
        // reactant, DROPPING any metabolite with coefficient exactly 0 — unlike
        // the forward/reverse branches (and `split`), it does not lump v==0 into
        // reactants. Mirror that exactly here instead of reusing split(true).
        if l < 0.0 && u > 0.0 {
            let mut reactants = Vec::new();
            let mut products = Vec::new();
            for (met, val) in coeffs {
                let v = val.as_f64().unwrap_or(0.0);
                if v > 0.0 {
                    products.push(met.clone());
                } else if v < 0.0 {
                    reactants.push(met.clone());
                }
                // v == 0.0: dropped, matching XGI's reversible branch.
            }
            if !(reactants.is_empty() && products.is_empty()) {
                let _ = dh.add_edge(
                    reactants.clone(),
                    products.clone(),
                    Some(rid.to_string()),
                    Value::Null,
                );
                let _ = dh.add_edge(
                    products,
                    reactants,
                    Some(format!("{rid}_reverse")),
                    Value::Null,
                );
            }
        }
    }
    Ok(dh)
}

/// Fetch a BIGG dataset over HTTP. Behind the `network` feature (OFF by default).
#[cfg(feature = "network")]
pub fn load_bigg_data(_dataset: &str) -> Result<DiHypergraph, ReadWriteError> {
    Err(ReadWriteError::Format(
        "load_bigg_data HTTP fetch not implemented in Phase 4".into(),
    ))
}
