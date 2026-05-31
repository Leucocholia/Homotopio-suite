use std::{
    collections::BTreeSet,
    env, fs, io,
    path::{Path, PathBuf},
};

use homotopy_dsl::{compile, CompileOptions};
use homotopy_model::serialize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct PresetToml {
    id: String,
    title: String,
    category: String,
    description: String,
    author: String,
    license: String,
    min_app_version: String,
    tags: Vec<String>,
    axioms: Vec<String>,
    constructed: Vec<String>,
    source: String,
    didactic: String,
    proof: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CommunityIndex {
    version: u32,
    presets: Vec<CommunityPreset>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CommunityPreset {
    id: String,
    title: String,
    category: String,
    description: String,
    author: String,
    license: String,
    min_app_version: String,
    tags: Vec<String>,
    axioms: Vec<String>,
    constructed: Vec<String>,
    source: String,
    didactic: Option<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut root = PathBuf::from("community-library");
    let mut check = false;
    for arg in env::args().skip(1) {
        if arg == "--check" {
            check = true;
        } else {
            root = PathBuf::from(arg);
        }
    }

    let index = build_index(&root)?;
    let generated = root.join("generated").join("index.json");
    let json = serde_json::to_string_pretty(&index)
        .map_err(|error| format!("could not serialize community index: {error}"))?
        + "\n";

    if check {
        let existing = fs::read_to_string(&generated).map_err(|error| {
            format!(
                "could not read generated catalog `{}`: {error}",
                generated.display()
            )
        })?;
        if existing != json {
            return Err(format!(
                "generated catalog `{}` is stale; rerun `cargo run -p homotopy-dsl --bin homotopy-library -- {}`",
                generated.display(),
                root.display()
            ));
        }
    } else {
        if let Some(parent) = generated.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "could not create generated catalog directory `{}`: {error}",
                    parent.display()
                )
            })?;
        }
        fs::write(&generated, json).map_err(|error| {
            format!(
                "could not write generated catalog `{}`: {error}",
                generated.display()
            )
        })?;
    }

    println!("validated {} community preset(s)", index.presets.len());
    Ok(())
}

fn build_index(root: &Path) -> Result<CommunityIndex, String> {
    let presets_root = root.join("presets");
    let mut presets = Vec::new();
    match fs::read_dir(&presets_root) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(|error| {
                    format!("could not read `{}` entry: {error}", presets_root.display())
                })?;
                if entry
                    .file_type()
                    .map_err(|error| {
                        format!("could not inspect `{}`: {error}", entry.path().display())
                    })?
                    .is_dir()
                {
                    presets.push(validate_preset(&entry.path())?);
                }
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "could not read presets directory `{}`: {error}",
                presets_root.display()
            ));
        }
    }
    presets.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(CommunityIndex {
        version: 1,
        presets,
    })
}

fn validate_preset(path: &Path) -> Result<CommunityPreset, String> {
    let metadata_path = path.join("preset.toml");
    let metadata = fs::read_to_string(&metadata_path)
        .map_err(|error| format!("could not read `{}`: {error}", metadata_path.display()))?;
    let metadata: PresetToml = toml::from_str(&metadata)
        .map_err(|error| format!("could not parse `{}`: {error}", metadata_path.display()))?;

    let folder_id = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("preset path `{}` has no folder name", path.display()))?;
    if metadata.id != folder_id {
        return Err(format!(
            "preset `{}` id does not match folder `{folder_id}`",
            metadata.id
        ));
    }
    if !is_slug(&metadata.id) {
        return Err(format!(
            "preset id `{}` is not a lowercase slug",
            metadata.id
        ));
    }

    let source_path = path.join(&metadata.source);
    let source = fs::read_to_string(&source_path)
        .map_err(|error| format!("could not read `{}`: {error}", source_path.display()))?;
    let didactic_path = path.join(&metadata.didactic);
    let didactic = fs::read_to_string(&didactic_path)
        .map_err(|error| format!("could not read `{}`: {error}", didactic_path.display()))?;

    let result = compile(&source, CompileOptions::default());
    if !result.is_ok() {
        return Err(format!(
            "preset `{}` did not compile: {:?}",
            metadata.id, result.diagnostics
        ));
    }
    let proof = result
        .proof
        .ok_or_else(|| format!("preset `{}` produced no proof", metadata.id))?;
    for generator in proof.signature.iter() {
        generator.diagram.check(true).map_err(|error| {
            format!(
                "preset `{}` signature item `{}` failed validation: {error:?}",
                metadata.id, generator.name
            )
        })?;
    }
    if let Some(workspace) = &proof.workspace {
        workspace.diagram.check(true).map_err(|error| {
            format!(
                "preset `{}` workspace failed validation: {error:?}",
                metadata.id
            )
        })?;
    }

    let proof_bytes = serialize::serialize(
        proof.signature.clone(),
        proof.workspace.clone(),
        proof.metadata.clone(),
    );
    serialize::deserialize(&proof_bytes)
        .ok_or_else(|| format!("preset `{}` failed .hom round-trip", metadata.id))?;

    let compiled_axioms: BTreeSet<_> = proof
        .signature
        .iter()
        .map(|info| info.name.clone())
        .collect();
    let declared_axioms: BTreeSet<_> = metadata.axioms.iter().cloned().collect();
    if compiled_axioms != declared_axioms {
        return Err(format!(
            "preset `{}` declared axioms {:?}, but compiled axioms are {:?}",
            metadata.id, declared_axioms, compiled_axioms
        ));
    }

    let compiled_constructed: BTreeSet<_> = result
        .symbols
        .iter()
        .map(|symbol| symbol.name.clone())
        .filter(|name| !compiled_axioms.contains(name))
        .collect();
    let declared_constructed: BTreeSet<_> = metadata.constructed.iter().cloned().collect();
    if compiled_constructed != declared_constructed {
        return Err(format!(
            "preset `{}` declared constructed symbols {:?}, but compiled constructed symbols are {:?}",
            metadata.id, declared_constructed, compiled_constructed
        ));
    }

    if let Some(proof_name) = &metadata.proof {
        let proof_path = path.join(proof_name);
        let proof = fs::read(&proof_path)
            .map_err(|error| format!("could not read `{}`: {error}", proof_path.display()))?;
        serialize::deserialize(&proof).ok_or_else(|| {
            format!(
                "optional proof file `{}` could not be deserialized",
                proof_path.display()
            )
        })?;
    }

    Ok(CommunityPreset {
        id: metadata.id,
        title: metadata.title,
        category: metadata.category,
        description: metadata.description,
        author: metadata.author,
        license: metadata.license,
        min_app_version: metadata.min_app_version,
        tags: metadata.tags,
        axioms: metadata.axioms,
        constructed: metadata.constructed,
        source,
        didactic: Some(didactic),
    })
}

fn is_slug(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
}
