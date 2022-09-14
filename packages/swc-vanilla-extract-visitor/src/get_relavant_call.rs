use std::collections::HashMap;

use swc_core::ecma::ast::{CallExpr, Callee, Expr, Ident, MemberProp};

use crate::constants::STYLE_FUNCTIONS;

pub fn get_relavant_call(
    call_expr: &CallExpr,
    namespace_import: &Option<Ident>,
    import_identifiers: &HashMap<Ident, String>,
) -> Option<String> {
    let callee = &call_expr.callee;

    if let Some(namespace_import) = namespace_import {
        if let Callee::Expr(expr) = callee {
            if !expr.is_member() {
                return None;
            }

            if let Expr::Member(member_expr) = &**expr {
                if let Expr::Ident(ident) = &*member_expr.obj {
                    if ident.sym == namespace_import.sym {
                        return STYLE_FUNCTIONS
                            .iter()
                            .find(|export_name| {
                                if let MemberProp::Ident(ident) = &member_expr.prop {
                                    &*ident.sym == **export_name
                                } else {
                                    false
                                }
                            })
                            .map(|v| v.to_string());
                    }
                }
            }
        }
        return None;
    } else {
        let import_info = import_identifiers.keys().find(|ident| {
            if let Callee::Expr(expr) = callee {
                if let Expr::Ident(expr_ident) = &**expr {
                    expr_ident.sym == ident.sym
                } else {
                    false
                }
            } else {
                false
            }
        });

        return import_info
            .map(|key| import_identifiers.get(key))
            .flatten()
            .cloned();
    }
}
