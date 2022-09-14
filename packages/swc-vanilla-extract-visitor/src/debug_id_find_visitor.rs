use std::collections::HashMap;

use swc_core::{
    common::pass::AstNodePath,
    ecma::{
        ast::{
            CallExpr, Callee, Expr, Ident, Lit, ObjectPatProp, Pat, Prop, PropName, PropOrSpread,
        },
        visit::{AstParentNodeRef, VisitAstPath, VisitWithPath},
    },
};

use crate::{
    constants::{DEBUGGABLE_FUNCTION_CONFIG, FILE_SCOPE_PACKAGE_IDENTIFIER},
    get_relavant_call::get_relavant_call,
};

/// A visitor to find corresponding debug id for the given callexpr, if it's a vanilla-extract style function.
pub struct DebugIdFindVisitor {
    pub is_compiled: bool,
    pub debug_id: Option<String>,

    namespace_import: Option<Ident>,
    import_identifiers: HashMap<Ident, String>,
}

impl DebugIdFindVisitor {
    pub fn new(
        namespace_import: Option<Ident>,
        import_identifiers: HashMap<Ident, String>,
    ) -> Self {
        Self {
            debug_id: None,
            is_compiled: false,

            namespace_import,
            import_identifiers,
        }
    }
}

fn extract_name<'r>(node: AstParentNodeRef<'r>) -> Option<String> {
    match node {
        AstParentNodeRef::PropOrSpread(prop, _) => {
            if let PropOrSpread::Prop(prop) = prop {
                if let Prop::KeyValue(key_value) = &**prop {
                    if let PropName::Ident(ident) = &key_value.key {
                        return Some(ident.sym.to_string());
                    }
                }
            }
        }
        AstParentNodeRef::ObjectPatProp(pat_prop, _) => {
            if let ObjectPatProp::KeyValue(key_value) = pat_prop {
                if let PropName::Ident(ident) = &key_value.key {
                    return Some(ident.sym.to_string());
                }
            }
        }
        AstParentNodeRef::VarDeclarator(declarator, _) => {
            match &declarator.name {
                Pat::Ident(ident) => {
                    return Some(ident.sym.to_string());
                }
                Pat::Array(array) => {
                    if let Some(elem) = &array.elems[0] {
                        if let Pat::Ident(ident) = &*elem {
                            return Some(ident.sym.to_string());
                        }
                    }
                }
                _ => return None,
            };
        }
        AstParentNodeRef::FnDecl(fn_decl, _) => {
            return Some(fn_decl.ident.sym.to_string());
        }
        AstParentNodeRef::ModuleDecl(module_decl, _) => {
            if module_decl.is_export_default_expr() || module_decl.is_export_default_decl() {
                return Some("default".to_string());
            }
        }
        _ => {}
    };

    None
}

fn get_debug_id<'r>(ast_path: &mut AstNodePath<AstParentNodeRef<'r>>) -> Option<String> {
    // When we arrived here, we no longer cares about keeping ast_path in sync, will just mutate it.

    let first_relevant_parent = ast_path.last().clone();

    if let Some(first_relevant_parent) = first_relevant_parent {
        // Special case: Handle `export const [themeClass, vars] = createTheme({});`
        // when it's already been compiled into this:
        //
        // var _createTheme = createTheme({}),
        //   _createTheme2 = _slicedToArray(_createTheme, 2),
        //   themeClass = _createTheme2[0],
        //   vars = _createTheme2[1];
        let parent = ast_path.last().clone(); //do not take, if this condition doesn't match we'll need to reuse last marker
        if let Some(parent) = parent {
            if let AstParentNodeRef::VarDecl(decl, _) = parent {
                if decl.decls.len() == 4 {
                    let theme_declarator = decl.decls.get(0).expect("Should exists");
                    let class_name_declarator = decl.decls.get(2).expect("Should exists");

                    let valid_theme_decl =
                        if let Some(theme_declarator_init) = theme_declarator.init.as_ref() {
                            if let Expr::Call(call) = &**theme_declarator_init {
                                if let Callee::Expr(callee) = &call.callee {
                                    if let Expr::Ident(callee_ident) = &**callee {
                                        "createTheme" == &*callee_ident.sym
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                    if valid_theme_decl {
                        if let Pat::Ident(class_name_decl_ident) = &class_name_declarator.name {
                            return Some(class_name_decl_ident.sym.to_string());
                        }
                    }
                }
            }
        }

        return match first_relevant_parent {
            AstParentNodeRef::ObjectPatProp(..)
            | AstParentNodeRef::Stmt(..)
            | AstParentNodeRef::Expr(..)
            | AstParentNodeRef::SpreadElement(..) => {
                let mut names = vec![];

                for path in ast_path.iter().rev() {
                    let name = extract_name(*path);
                    if let Some(name) = name {
                        names.insert(0, name);
                    }
                }

                if names.len() > 0 {
                    Some(names.join("_").to_string())
                } else {
                    None
                }
            }
            _ => extract_name(*first_relevant_parent),
        };
    }

    None
}

impl VisitAstPath for DebugIdFindVisitor {
    fn visit_call_expr<'ast: 'r, 'r>(
        &mut self,
        call_expr: &'r CallExpr,
        ast_path: &mut AstNodePath<AstParentNodeRef<'r>>,
    ) {
        if self.is_compiled {
            return;
        }

        if let Callee::Expr(expr) = &call_expr.callee {
            if let Expr::Ident(ident) = &**expr {
                if &*ident.sym == "require" {
                    if let Some(arg) = call_expr.args.get(0) {
                        if let Expr::Lit(expr) = &*arg.expr {
                            if let Lit::Str(expr) = expr {
                                if &*expr.value == FILE_SCOPE_PACKAGE_IDENTIFIER {
                                    // If file scope import is found it means the file has already been compiled
                                    self.is_compiled = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        let used_export =
            get_relavant_call(call_expr, &self.namespace_import, &self.import_identifiers);

        if let Some(used_export) = used_export {
            if let Some(max_params) = DEBUGGABLE_FUNCTION_CONFIG.get(&used_export) {
                if call_expr.args.len() < *max_params {
                    self.debug_id = get_debug_id(ast_path);
                }
            }
        }

        call_expr.visit_children_with_path(self, ast_path);
    }
}
