use std::fs;
use std::path::Path;

use super::structs::{BackendOption, ModelDef, Task};

// scan the models directory for folders containing model.toml
pub fn scan_models(models_dir: &Path) -> Vec<ModelDef> {
    let mut models = Vec::new();

    if !models_dir.exists() {
        warn!("Models directory not found: {:?}", models_dir);
        return models;
    }

    let entries = match fs::read_dir(models_dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to read models dir: {}", e);
            return models;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let toml_path = path.join("model.toml");
        if !toml_path.exists() {
            continue;
        }

        match load_model_def(&toml_path, &path) {
            Ok(def) => {
                info!(
                    "Found model: {} ({}) - tasks: {:?}",
                    def.name, def.id, def.tasks
                );
                models.push(def);
            }
            Err(e) => warn!("Failed to load model from {:?}: {}", path, e),
        }
    }

    models
}

fn load_model_def(toml_path: &Path, model_dir: &Path) -> Result<ModelDef, String> {
    let content = fs::read_to_string(toml_path).map_err(|e| format!("read error: {}", e))?;

    let parsed: ModelToml = toml::from_str(&content).map_err(|e| format!("parse error: {}", e))?;

    let mut def = parsed.model;
    def.path = model_dir.to_path_buf();

    // Reject models whose primary binary is a Git LFS pointer — the file has not
    // been downloaded yet and will fail to parse as ONNX/protobuf at load time.
    let onnx_path = model_dir.join("model.onnx");
    if onnx_path.exists() && is_lfs_pointer(&onnx_path) {
        return Err("model.onnx is a Git LFS pointer (binary not downloaded). \
             Run `git lfs pull` to download the actual model file."
            .to_string());
    }

    Ok(def)
}

/// Returns true if the file starts with the Git LFS pointer magic prefix.
/// Reads only the first 23 bytes — avoids loading large model binaries.
fn is_lfs_pointer(path: &Path) -> bool {
    use std::io::Read;
    const LFS_MAGIC: &[u8] = b"version https://git-lfs";
    let mut buf = [0u8; 23];
    match fs::File::open(path) {
        Ok(mut f) => {
            let n = f.read(&mut buf).unwrap_or(0);
            &buf[..n] == LFS_MAGIC
        }
        Err(_) => false,
    }
}

#[derive(serde::Deserialize)]
struct ModelToml {
    model: ModelDef,
}

// Code backends per task
pub fn code_backends(task: Task) -> Vec<BackendOption> {
    match task {
        Task::Intent => vec![BackendOption {
            id: "intent-classifier".into(),
            name: "Intent Classifier".into(),
            model_id: None,
        }],
        Task::Slots => vec![],
        Task::Vad => vec![
            BackendOption {
                id: "energy".into(),
                name: "Energy-based".into(),
                model_id: None,
            },
            BackendOption {
                id: "nnnoiseless".into(),
                name: "Nnnoiseless".into(),
                model_id: None,
            },
        ],
        Task::NoiseSuppression => vec![BackendOption {
            id: "nnnoiseless".into(),
            name: "Nnnoiseless".into(),
            model_id: None,
        }],
        Task::Stt => vec![BackendOption {
            id: "vosk".into(),
            name: "Vosk".into(),
            model_id: None,
        }],
    }
}

// get all available options for a task:
// "none" first, then code backends, then AI models from catalog
pub fn get_options(task: Task, models: &[ModelDef]) -> Vec<BackendOption> {
    let mut options = vec![BackendOption {
        id: "none".into(),
        name: "Disabled".into(),
        model_id: None,
    }];

    options.extend(code_backends(task));

    for model in models {
        if model.tasks.contains(&task) {
            options.push(BackendOption {
                id: model.id.clone(),
                name: model.name.clone(),
                model_id: Some(model.id.clone()),
            });
        }
    }

    options
}

#[cfg(test)]
mod tests {
    use super::*;
    fn write_model_toml(dir: &std::path::Path) {
        let toml = "[model]\nid = \"test-model\"\nname = \"Test\"\ntasks = [\"intent\"]\n";
        std::fs::write(dir.join("model.toml"), toml).unwrap();
    }

    /// scan_models must skip a model whose model.onnx is a Git LFS pointer.
    #[test]
    fn scan_models_skips_lfs_pointer_onnx() {
        let dir = tempfile::tempdir().unwrap();
        let model_dir = dir.path().join("lfs-model");
        std::fs::create_dir_all(&model_dir).unwrap();
        write_model_toml(&model_dir);
        // write an LFS pointer instead of a real binary
        std::fs::write(
            model_dir.join("model.onnx"),
            b"version https://git-lfs.github.com/spec/v1\noid sha256:abc123\nsize 235052644\n",
        )
        .unwrap();

        let models = scan_models(dir.path());
        assert!(
            models.is_empty(),
            "scan_models should exclude models with LFS pointer onnx files, got: {:?}",
            models.iter().map(|m| &m.id).collect::<Vec<_>>()
        );
    }

    /// scan_models must include a model whose model.onnx starts with real binary bytes.
    #[test]
    fn scan_models_includes_real_onnx() {
        let dir = tempfile::tempdir().unwrap();
        let model_dir = dir.path().join("real-model");
        std::fs::create_dir_all(&model_dir).unwrap();
        write_model_toml(&model_dir);
        // write a fake "real" binary (starts with non-LFS bytes)
        std::fs::write(model_dir.join("model.onnx"), b"\x08\x07\x12\x07pytorch").unwrap();

        let models = scan_models(dir.path());
        assert_eq!(
            models.len(),
            1,
            "scan_models should include model with real onnx binary"
        );
        assert_eq!(models[0].id, "test-model");
    }

    /// scan_models must include a model that has no model.onnx at all
    /// (non-embedding models like vosk do not have .onnx files).
    #[test]
    fn scan_models_includes_model_without_onnx() {
        let dir = tempfile::tempdir().unwrap();
        let model_dir = dir.path().join("vosk-model");
        std::fs::create_dir_all(&model_dir).unwrap();
        write_model_toml(&model_dir);
        // no model.onnx — should not be filtered

        let models = scan_models(dir.path());
        assert_eq!(
            models.len(),
            1,
            "scan_models should include models without a .onnx file"
        );
    }
}

pub fn is_valid_backend(task: Task, backend_id: &str, models: &[ModelDef]) -> bool {
    if backend_id == "none" {
        return true;
    }

    if code_backends(task).iter().any(|b| b.id == backend_id) {
        return true;
    }

    models
        .iter()
        .any(|m| m.id == backend_id && m.tasks.contains(&task))
}
