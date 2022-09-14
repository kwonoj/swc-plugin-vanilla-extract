use serde_json::Value;
use swc_core::{
    ecma::{ast::Program, visit::*},
    plugin::{
        metadata::TransformPluginMetadataContextKind, plugin_transform,
        proxies::TransformPluginProgramMetadata,
    },
};

use swc_vanilla_extract_visitor::create_extract_visitor;

#[plugin_transform]
pub fn process(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let filename = metadata.get_context(&TransformPluginMetadataContextKind::Filename);
    let filename = if let Some(filename) = filename.as_deref() {
        filename
    } else {
        "unknown.js"
    };

    let cwd = metadata.get_context(&TransformPluginMetadataContextKind::Cwd);
    let cwd = if let Some(cwd) = cwd.as_deref() {
        cwd
    } else {
        "."
    };

    let config = metadata.get_transform_plugin_config();
    let package_name = if let Some(config) = config {
        let config: Value = serde_json::from_str(&config).expect("Config should be serializable");

        let pkg_name = config["packageName"].as_str();
        if let Some(pkg_name) = pkg_name {
            pkg_name.to_string()
        } else {
            "swc-plugin-vanilla-extract".to_string()
        }
    } else {
        "swc-plugin-vanilla-extract".to_string()
    };

    let visitor = create_extract_visitor(
        std::sync::Arc::new(metadata.source_map),
        metadata.comments.as_ref(),
        filename,
        &package_name,
        cwd,
    );

    program.fold_with(&mut as_folder(visitor))
}
