use std::collections::{HashSet, HashMap};

use once_cell::sync::Lazy;
use swc_core::{
    common::{comments::Comments, SourceMapper},
    ecma::{
        ast::{ExportDecl, ImportDecl, ImportSpecifier, ModuleExportName, Ident},
        visit::{noop_visit_mut_type, VisitMut},
    },
};

static FILE_SCOPE_PACKAGE_IDENTIFIER: &str = "@vanilla-extract/css/fileScope";

static PACKAGE_IDENTIFIERS: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert("@vanilla-extract/css".to_string());
    set.insert("@vanilla-extract/recipes".to_string());
    set
});

static STYLE_FUNCTIONS: [&str; 6] = [
    "globalStyle",
    "createGlobalTheme",
    "createThemeContract",
    "globalFontFace",
    "globalKeyframes",
    "recipe",
];

pub struct VanillaExtractVisitor {
    is_esm: bool,
    is_css_file: bool,
    is_compiled: bool,

    namespace_import: Option<Ident>,
    import_identifiers: HashMap<Ident, String>,
}

impl VanillaExtractVisitor {
    pub fn new() -> Self {
        VanillaExtractVisitor {
            is_esm: false,
            is_css_file: false,
            is_compiled: false,

            namespace_import: None,
            import_identifiers: Default::default()
        }
    }
}

impl VisitMut for VanillaExtractVisitor {
    noop_visit_mut_type!();

    fn visit_mut_import_decl(&mut self, import_decl: &mut ImportDecl) {
        self.is_esm = true;

        if !self.is_css_file || self.is_compiled {
            // Bail early if file isn't a .css.ts file or the file has already been compiled
            return;
        }

        let src = &*import_decl.src.value;
        if src == FILE_SCOPE_PACKAGE_IDENTIFIER {
            // If file scope import is found it means the file has already been compiled
            self.is_compiled = true;
            return;
        } else if PACKAGE_IDENTIFIERS.contains(src) {
            for specifier in &import_decl.specifiers {
                match specifier {
                    ImportSpecifier::Named(named_specifier) => {
                        let imported = &named_specifier.imported;
                        let local = &named_specifier.local;

                        if let Some(imported) = imported {
                            let import_name = match imported {
                                ModuleExportName::Ident(ident) => &*ident.sym,
                                ModuleExportName::Str(str) => &*str.value,
                            };

                            if STYLE_FUNCTIONS.contains(&import_name) {
                                self.import_identifiers.insert(local.clone(), import_name.to_string());
                            }
                        }
                    }
                    ImportSpecifier::Default(default_specifier) => {
                        //noop
                    }
                    ImportSpecifier::Namespace(namespace_specifier) => {
                        self.namespace_import = Some(namespace_specifier.local.clone());
                    }
                }
            }

            /*path.node.specifiers.forEach((specifier) => {
              if (t.isImportNamespaceSpecifier(specifier)) {
                this.namespaceImport = specifier.local.name;
              } else if (t.isImportSpecifier(specifier)) {
                const { imported, local } = specifier;

                const importName = (
                  t.isIdentifier(imported) ? imported.name : imported.value
                ) as StyleFunction;

                if (styleFunctions.includes(importName)) {
                  this.importIdentifiers.set(local.name, importName);
                }
              }
            });*/
        }
    }

    fn visit_mut_export_decl(&mut self, _: &mut ExportDecl) {
        self.is_esm = true;
    }
}

pub fn create_extract_visitor<C: Clone + Comments, S: SourceMapper>(
    _source_map: std::sync::Arc<S>,
    _comments: C,
    _filename: String,
) -> VanillaExtractVisitor {
    VanillaExtractVisitor::new()
}
