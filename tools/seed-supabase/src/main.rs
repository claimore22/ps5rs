use std::collections::{BTreeSet, HashMap};
use std::path::Path;

fn main() {
    let input_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("data")
        .join("nids.csv");

    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("output");

    eprintln!("Reading NID catalog from {}", input_path.display());

    let content = match std::fs::read_to_string(&input_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to read {}: {e}", input_path.display());
            std::process::exit(1);
        }
    };

    let mut entries: HashMap<String, (String, BTreeSet<String>)> = HashMap::new();

    for (lineno, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((nid, name)) = line.split_once(char::is_whitespace) else {
            eprintln!("warning: skipping malformed line {}: {line}", lineno + 1);
            continue;
        };

        let nid = nid.trim();
        let name = name.trim();
        if nid.is_empty() || name.is_empty() {
            continue;
        }

        let entry = entries.entry(nid.to_string()).or_insert_with(|| {
            let u64_val = ps5_nid::nid_to_u64(nid).unwrap_or(0);
            (u64_val.to_string(), BTreeSet::new())
        });
        entry.1.insert(name.to_string());
    }

    eprintln!("Parsed {} unique NID entries", entries.len());

    let entries_path = output_dir.join("nid_entries.csv");
    let mut w_entries = csv::Writer::from_path(&entries_path).unwrap_or_else(|e| {
        eprintln!("error: failed to create {}: {e}", entries_path.display());
        std::process::exit(1);
    });
    w_entries.write_record(["nid", "nid_u64", "primary_name"]).unwrap();

    let names_path = output_dir.join("nid_names.csv");
    let mut w_names = csv::Writer::from_path(&names_path).unwrap_or_else(|e| {
        eprintln!("error: failed to create {}: {e}", names_path.display());
        std::process::exit(1);
    });
    w_names
        .write_record(["nid", "name", "library", "source", "confidence", "evidence"])
        .unwrap();

    let null = "\\N";

    for (nid, (u64_str, names)) in &entries {
        let primary_name = names.first().map(|s| s.as_str()).unwrap_or("");
        w_entries
            .write_record([nid.as_str(), u64_str.as_str(), primary_name])
            .unwrap();

        for name in names {
            w_names
                .write_record([
                    nid.as_str(),
                    name.as_str(),
                    null,
                    "ps5rs-builtin",
                    "100",
                    null,
                ])
                .unwrap();
        }
    }

    w_entries.flush().unwrap();
    w_names.flush().unwrap();

    let name_count: usize = entries.values().map(|(_, names)| names.len()).sum();

    eprintln!(
        "Generated {}nid_entries.csv ({} rows)",
        output_dir.display().to_string() + "\\",
        entries.len()
    );
    eprintln!(
        "Generated {}nid_names.csv ({} rows)",
        output_dir.display().to_string() + "\\",
        name_count
    );
}
