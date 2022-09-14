use std::collections::HashMap;

use swc_core::ecma::{
    ast::{ExportDecl, Ident, ImportDecl, ImportSpecifier, ModuleExportName},
    visit::Visit,
};

use crate::constants::{FILE_SCOPE_PACKAGE_IDENTIFIER, PACKAGE_IDENTIFIERS, STYLE_FUNCTIONS};

/// A visitor to collect imports from vanilla-extract packages
pub struct ImportCollectVisitor {
    pub is_esm: bool,
    pub is_compiled: bool,

    pub namespace_import: Option<Ident>,
    pub import_identifiers: HashMap<Ident, String>,
}

impl ImportCollectVisitor {
    pub fn new() -> Self {
        Self {
            is_esm: false,
            is_compiled: false,

            namespace_import: None,
            import_identifiers: Default::default(),
        }
    }
}

impl Visit for ImportCollectVisitor {
    fn visit_import_decl(&mut self, import_decl: &ImportDecl) {
        self.is_esm = true;

        if self.is_compiled {
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
                                self.import_identifiers
                                    .insert(local.clone(), import_name.to_string());
                            }
                        } else if STYLE_FUNCTIONS.contains(&&*local.sym) {
                            self.import_identifiers
                                .insert(local.clone(), local.sym.to_string());
                        }
                    }
                    ImportSpecifier::Default(_default_specifier) => {
                        //noop
                    }
                    ImportSpecifier::Namespace(namespace_specifier) => {
                        self.namespace_import = Some(namespace_specifier.local.clone());
                    }
                }
            }
        }
    }

    fn visit_export_decl(&mut self, _: &ExportDecl) {
        self.is_esm = true;
    }
}
