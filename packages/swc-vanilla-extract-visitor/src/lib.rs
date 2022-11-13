use constants::{CSS_FILE_FILTER_REGEX, FILE_SCOPE_IMPORT_NAME, FILE_SCOPE_PACKAGE_IDENTIFIER};
use debug_id_find_visitor::DebugIdFindVisitor;
use debug_id_inject_visitor::DebugIdInjectVisitor;
use import_collect_visitor::ImportCollectVisitor;
use path_slash::PathBufExt as _;
use std::path::PathBuf;

use swc_core::{
    common::{comments::Comments, SourceMapper, DUMMY_SP},
    ecma::{
        ast::{
            CallExpr, Callee, Expr, ExprOrSpread, ExprStmt, Ident, ImportDecl, ImportSpecifier,
            ImportStarAsSpecifier, Lit, MemberExpr, MemberProp, ModuleDecl, ModuleItem, Stmt, Str,
        },
        atoms::JsWord,
        visit::{VisitMut, VisitMutWith, VisitWith, VisitWithPath},
    },
    quote,
};

mod constants;
mod debug_id_find_visitor;
mod debug_id_inject_visitor;
mod get_relavant_call;
mod import_collect_visitor;

/// Top level visitor for vanilla-extract plugin.
pub struct VanillaExtractVisitor {
    file_path: String,
    package_name: String,

    is_css_file: bool,
}

impl VanillaExtractVisitor {
    pub fn new(filename: &str, package_name: &str, package_dir: &str) -> Self {
        VanillaExtractVisitor {
            file_path: PathBuf::from(package_dir)
                .join(filename)
                .to_slash_lossy()
                .to_string(),
            package_name: package_name.to_string(),

            is_css_file: CSS_FILE_FILTER_REGEX.is_match(filename),
        }
    }
}

impl VisitMut for VanillaExtractVisitor {
    fn visit_mut_module_items(
        &mut self,
        items: &mut Vec<ModuleItem>,
        //ast_path: &mut AstNodePath<AstParentNodeRef<'r>>,
    ) {
        // Bail early if file isn't a .css.ts file
        if !self.is_css_file {
            return;
        }

        let mut new_items = vec![];
        let mut import_collect_visitor = ImportCollectVisitor::new();

        // Runs all childrens with import collect visitor to collect related imports first
        for item in items.iter() {
            item.visit_children_with(&mut import_collect_visitor);
        }

        if import_collect_visitor.is_compiled {
            return;
        }

        let mut debug_id_find_visitor = DebugIdFindVisitor::new(
            import_collect_visitor.namespace_import.clone(),
            import_collect_visitor.import_identifiers.clone(),
        );
        let mut debug_id_inject_visitor = DebugIdInjectVisitor::new(
            import_collect_visitor.namespace_import,
            import_collect_visitor.import_identifiers,
        );

        for mut item in items.drain(..) {
            // Bail early if file has already been compiled
            if !debug_id_find_visitor.is_compiled {
                // First, find debug id with ast_path visitor
                item.visit_children_with_path(&mut debug_id_find_visitor, &mut Default::default());
                // Inject debug id with mutable visitor. This make each node traverses twice, but
                // mutable visitor does not get the ast_path with node to read its debug id.
                debug_id_inject_visitor.debug_id = debug_id_find_visitor.debug_id.take();
                //We'll keep single inject visitor as stateful, visitor will consume debug_id if exists
                item.visit_mut_children_with(&mut debug_id_inject_visitor);
            }
            new_items.push(item);
        }
        *items = new_items;

        if !debug_id_find_visitor.is_compiled {
            // Wrap module with file scope calls

            // Plugin does not determine type of import to be CJS or ESM - SWC core should transpile
            // accordingly depends on the config.
            let import_scope = ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![ImportSpecifier::Namespace(ImportStarAsSpecifier {
                    span: DUMMY_SP,
                    local: FILE_SCOPE_IMPORT_NAME.clone(),
                })],
                src: Box::new(Str::from(FILE_SCOPE_PACKAGE_IDENTIFIER)),
                type_only: false,
                asserts: None,
            }));
            let set_file_scope = ModuleItem::Stmt(Stmt::Expr(ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(Expr::Ident(FILE_SCOPE_IMPORT_NAME.clone())),
                        prop: MemberProp::Ident(Ident::new("setFileScope".into(), DUMMY_SP)),
                    }))),
                    args: vec![
                        ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Str(Str::from(JsWord::from(
                                self.file_path.clone(),
                            ))))),
                        },
                        ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Str(Str::from(JsWord::from(
                                self.package_name.clone(),
                            ))))),
                        },
                    ],
                    type_args: None,
                })),
            }));

            items.insert(0, import_scope);
            items.insert(1, set_file_scope);

            items.push(quote!(
                "$file_scope_import_name.endFileScope()" as ModuleItem,
                file_scope_import_name = FILE_SCOPE_IMPORT_NAME.clone()
            ));
        }
    }
}

pub fn create_extract_visitor<C: Clone + Comments, S: SourceMapper>(
    _source_map: std::sync::Arc<S>,
    _comments: C,
    filename: &str,
    package_name: &str,
    package_dir: &str,
) -> VanillaExtractVisitor {
    VanillaExtractVisitor::new(filename, package_name, package_dir)
}
