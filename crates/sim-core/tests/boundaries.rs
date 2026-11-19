use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_CRATE_PREFIXES: &[&str] =
    &["wasm-bindgen", "web-sys", "js-sys", "gloo", "wasm-streams"];

const FORBIDDEN_SOURCE_TOKENS: &[&str] = &["wasm_bindgen", "web_sys", "js_sys"];

fn is_forbidden_crate(name: &str) -> bool {
    FORBIDDEN_CRATE_PREFIXES
        .iter()
        .any(|prefix| name == *prefix || name.starts_with(&format!("{prefix}-")))
}

fn parse_cargo_lock(lockfile: &str) -> BTreeMap<String, Vec<String>> {
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_name: Option<String> = None;
    let mut in_deps = false;

    for raw_line in lockfile.lines() {
        let line = raw_line.trim();
        if line == "[[package]]" {
            current_name = None;
            in_deps = false;
            continue;
        }
        if in_deps {
            if line.starts_with(']') {
                in_deps = false;
                continue;
            }
            if let Some(quoted) = line.strip_prefix('"') {
                if let Some(end) = quoted.find('"') {
                    let dep_spec = &quoted[..end];
                    if let Some(dep_name) = dep_spec.split_whitespace().next() {
                        if let Some(pkg) = &current_name {
                            graph.entry(pkg.clone()).or_default().push(dep_name.into());
                        }
                    }
                }
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("name = \"") {
            if let Some(name) = rest.strip_suffix('"') {
                current_name = Some(name.to_owned());
                graph.entry(name.to_owned()).or_default();
            }
        } else if line.starts_with("dependencies = [") {
            in_deps = !line.ends_with(']');
        }
    }
    graph
}

fn transitive_dependencies(graph: &BTreeMap<String, Vec<String>>, root: &str) -> BTreeSet<String> {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::new();
    if let Some(direct) = graph.get(root) {
        for dep in direct {
            if visited.insert(dep.clone()) {
                queue.push_back(dep.clone());
            }
        }
    }
    while let Some(pkg) = queue.pop_front() {
        if let Some(deps) = graph.get(&pkg) {
            for dep in deps {
                if visited.insert(dep.clone()) {
                    queue.push_back(dep.clone());
                }
            }
        }
    }
    visited
}

fn find_forbidden_dependencies(lockfile: &str, root: &str) -> Result<Vec<String>, String> {
    let graph = parse_cargo_lock(lockfile);
    if !graph.contains_key(root) {
        return Err(format!("Package '{root}' not found in Cargo.lock"));
    }
    let deps = transitive_dependencies(&graph, root);
    let forbidden: Vec<String> = deps
        .into_iter()
        .filter(|name| is_forbidden_crate(name))
        .collect();
    Ok(forbidden)
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("read sim-core src directory");
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn sim_core_has_no_browser_or_wasm_dependencies() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root");

    let lockfile = fs::read_to_string(workspace_root.join("Cargo.lock")).expect("read Cargo.lock");
    let graph = parse_cargo_lock(&lockfile);
    let sim_core_deps = transitive_dependencies(&graph, "sim-core");
    assert!(
        sim_core_deps.contains("serde"),
        "expected serde in sim-core dependency graph; lockfile parsing may be out of date"
    );

    let forbidden = find_forbidden_dependencies(&lockfile, "sim-core").expect("parse Cargo.lock");
    assert!(
        forbidden.is_empty(),
        "sim-core must remain browser-independent, but depends on: {forbidden:?}"
    );

    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("read Cargo.toml");
    for prefix in FORBIDDEN_CRATE_PREFIXES {
        assert!(
            !manifest.contains(prefix),
            "crates/sim-core/Cargo.toml must not reference browser/WASM crate '{prefix}'"
        );
    }

    let mut rs_files = Vec::new();
    collect_rs_files(&manifest_dir.join("src"), &mut rs_files);
    assert!(!rs_files.is_empty(), "expected Rust source files in src/");
    for file in rs_files {
        let content = fs::read_to_string(&file).expect("read source file");
        for token in FORBIDDEN_SOURCE_TOKENS {
            assert!(
                !content.contains(token),
                "{} must not reference browser/WASM symbol '{token}'",
                file.display()
            );
        }
    }
}

#[test]
fn boundary_check_detects_direct_and_transitive_browser_dependencies() {
    let direct_violation = r#"
[[package]]
name = "sim-core"
dependencies = [
 "serde",
 "web-sys 0.3.70",
]
"#;
    assert_eq!(
        find_forbidden_dependencies(direct_violation, "sim-core").unwrap(),
        vec!["web-sys".to_owned()]
    );

    let transitive_violation = r#"
[[package]]
name = "sim-core"
dependencies = [
 "helper",
]

[[package]]
name = "helper"
dependencies = [
 "wasm-bindgen-futures",
]
"#;
    assert_eq!(
        find_forbidden_dependencies(transitive_violation, "sim-core").unwrap(),
        vec!["wasm-bindgen-futures".to_owned()]
    );
}
