use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

/// The structure of the sidecar decisions file created by the LLM
#[derive(Debug, Deserialize)]
pub struct DecisionManifest {
    pub version: u32,
    pub decisions: HashMap<String, Decision>,
}

#[derive(Debug, Deserialize)]
pub struct Decision {
    pub action: String, // "keep" or "drop"
    pub category: Option<String>,
}

pub fn execute(decisions_path: PathBuf) -> Result<()> {
    eprintln!(
        "Applying curation decisions from {}",
        decisions_path.display()
    );

    // 1. Read decisions manifest
    let decisions_content =
        fs::read_to_string(&decisions_path).context("Failed to read decisions file")?;
    let manifest: DecisionManifest =
        serde_json::from_str(&decisions_content).context("Failed to parse decisions.json")?;

    if manifest.version != 1 {
        eprintln!("Warning: Unknown decisions version {}", manifest.version);
    }

    // 2. Read canonical JSON pipe from stdin
    let mut stdin_content = String::new();
    io::stdin()
        .read_to_string(&mut stdin_content)
        .context("Failed to read from stdin")?;

    let mut report: serde_json::Value =
        serde_json::from_str(&stdin_content).context("Failed to parse JSON report from stdin")?;

    // We extract the `entries` array and modify it
    if let Some(entries_array) = report.get_mut("entries").and_then(|v| v.as_array_mut()) {
        let mut curated_entries = Vec::new();

        for entry_val in entries_array.iter_mut() {
            let mut keep = false;

            // Try to extract ID to look it up, optionally parse the whole entry
            if let Some(id) = entry_val.get("id").and_then(|v| v.as_str()) {
                if let Some(decision) = manifest.decisions.get(id) {
                    if decision.action == "keep" {
                        keep = true;
                        // Map the category back to the entry
                        if let Some(ref cat) = decision.category {
                            if let serde_json::Value::Object(map) = entry_val {
                                map.insert(
                                    "category".to_string(),
                                    serde_json::Value::String(cat.clone()),
                                );
                            }
                        }
                    } else if decision.action == "drop" {
                        keep = false;
                    }
                } else {
                    // Implicit fallback for missing decisions: keep it as 'Ignored' or 'Investigation'
                    // For now, if the LLM completely omits it, we drop it to keep the PR clean.
                    keep = false;
                }
            } else {
                // If an entry lacks an ID entirely (legacy?), drop it or keep it?
                // Default to keep for safety if ID strategy gets temporarily broken.
                keep = true;
            }

            if keep {
                curated_entries.push(entry_val.clone());
            }
        }

        eprintln!(
            "  * Curated from {} to {} entries",
            entries_array.len(),
            curated_entries.len()
        );
        *entries_array = curated_entries;
    }

    // 3. Serialize back out to stdout
    let stdout_output = serde_json::to_string_pretty(&report)?;
    println!("{}", stdout_output);

    Ok(())
}
