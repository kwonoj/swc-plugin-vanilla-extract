use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;
use regex::Regex as Regexp;
use swc_core::{common::DUMMY_SP, ecma::ast::Ident};

pub static FILE_SCOPE_IMPORT_NAME: Lazy<Ident> =
    Lazy::new(|| Ident::new("__vanilla_filescope__".into(), DUMMY_SP));
pub static FILE_SCOPE_PACKAGE_IDENTIFIER: &str = "@vanilla-extract/css/fileScope";

pub static PACKAGE_IDENTIFIERS: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert("@vanilla-extract/css".to_string());
    set.insert("@vanilla-extract/recipes".to_string());
    set
});

pub static STYLE_FUNCTIONS: [&str; 14] = [
    "style",
    "createTheme",
    "styleVariants",
    "fontFace",
    "keyframes",
    "createVar",
    "recipe",
    "createContainer",
    "globalStyle",
    "createGlobalTheme",
    "createThemeContract",
    "globalFontFace",
    "globalKeyframes",
    "recipe",
];

pub static CSS_FILE_FILTER_REGEX: Lazy<Regexp> =
    Lazy::new(|| Regexp::new(r"\.css\.(js|mjs|jsx|ts|tsx)(\?used)?$").unwrap());

pub static DEBUGGABLE_FUNCTION_CONFIG: Lazy<HashMap<String, usize>> = Lazy::new(|| {
    let mut map = HashMap::default();
    map.insert("style".to_string(), 2);
    map.insert("createTheme".to_string(), 3);
    map.insert("styleVariants".to_string(), 3);
    map.insert("fontFace".to_string(), 2);
    map.insert("keyframes".to_string(), 2);
    map.insert("createVar".to_string(), 1);
    map.insert("recipe".to_string(), 2);
    map.insert("createContainer".to_string(), 1);
    map
});
