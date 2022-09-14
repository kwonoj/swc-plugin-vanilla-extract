use std::collections::HashMap;

use swc_core::ecma::{
    ast::{CallExpr, Expr, ExprOrSpread, Ident, Lit, Str},
    visit::{VisitMut, VisitMutWith},
};

use crate::{constants::DEBUGGABLE_FUNCTION_CONFIG, get_relavant_call::get_relavant_call};

/// A visitor actually injects debugid into given callexpr, if given call expr is a vanilla-extract style function.
pub struct DebugIdInjectVisitor {
    pub debug_id: Option<String>,
    namespace_import: Option<Ident>,
    import_identifiers: HashMap<Ident, String>,
}

impl DebugIdInjectVisitor {
    pub fn new(
        namespace_import: Option<Ident>,
        import_identifiers: HashMap<Ident, String>,
    ) -> Self {
        DebugIdInjectVisitor {
            debug_id: None,
            namespace_import,
            import_identifiers,
        }
    }
}

impl VisitMut for DebugIdInjectVisitor {
    fn visit_mut_call_expr(&mut self, call_expr: &mut CallExpr) {
        let used_export =
            get_relavant_call(call_expr, &self.namespace_import, &self.import_identifiers);

        if let Some(used_export) = used_export {
            if let Some(max_params) = DEBUGGABLE_FUNCTION_CONFIG.get(&used_export) {
                if call_expr.args.len() < *max_params {
                    if let Some(debug_id) = self.debug_id.take() {
                        call_expr.args.push(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Str(Str::from(debug_id)))),
                        })
                    }
                }
            }
        }

        call_expr.visit_mut_children_with(self);
    }
}
