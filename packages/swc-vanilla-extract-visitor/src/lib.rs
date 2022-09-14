use once_cell::sync::Lazy;
use path_slash::PathBufExt as _;
use regex::Regex as Regexp;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use swc_core::{
    common::{comments::Comments, SourceMapper, DUMMY_SP, pass::AstNodePath},
    ecma::{
        ast::{
            CallExpr, Callee, ExportDecl, Expr, ExprOrSpread, ExprStmt, Ident, ImportDecl,
            ImportSpecifier, ImportStarAsSpecifier, Lit, MemberExpr, MemberProp, ModuleDecl,
            ModuleExportName, ModuleItem, ObjectPatProp, Pat, PropName, Stmt, Str, Prop, PropOrSpread,
        },
        atoms::JsWord,
        visit::{
            AstParentNodeRef, Visit, VisitAstPath, VisitMut, VisitMutWith, VisitWith, VisitWithPath,
        },
    },
    quote,
};

static FILE_SCOPE_IMPORT_NAME: Lazy<Ident> =
    Lazy::new(|| Ident::new("__vanilla_filescope__".into(), DUMMY_SP));
static FILE_SCOPE_PACKAGE_IDENTIFIER: &str = "@vanilla-extract/css/fileScope";

static PACKAGE_IDENTIFIERS: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert("@vanilla-extract/css".to_string());
    set.insert("@vanilla-extract/recipes".to_string());
    set
});

static STYLE_FUNCTIONS: [&str; 14] = [
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

static CSS_FILE_FILTER_REGEX: Lazy<Regexp> =
    Lazy::new(|| Regexp::new(r"\.css\.(js|mjs|jsx|ts|tsx)(\?used)?$").unwrap());

static DEBUGGABLE_FUNCTION_CONFIG: Lazy<HashMap<String, usize>> = Lazy::new(|| {
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

fn get_relavant_call(
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
                        return STYLE_FUNCTIONS.iter().find(|export_name| {
                            if let MemberProp::Ident(ident) = &member_expr.prop {
                                &*ident.sym == **export_name
                            } else {
                                false
                            }
                        }).map(|v| v.to_string());
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

        return import_info.map(|key| import_identifiers.get(key)).flatten().cloned();
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
                src: Str::from(FILE_SCOPE_PACKAGE_IDENTIFIER),
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

struct ImportCollectVisitor {
    is_esm: bool,
    is_compiled: bool,

    namespace_import: Option<Ident>,
    import_identifiers: HashMap<Ident, String>,
}

impl ImportCollectVisitor {
    fn new() -> Self {
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

struct DebugIdFindVisitor {
    is_compiled: bool,
    debug_id: Option<String>,

    namespace_import: Option<Ident>,
    import_identifiers: HashMap<Ident, String>,
}

impl DebugIdFindVisitor {
    fn new(namespace_import: Option<Ident>, import_identifiers: HashMap<Ident, String>) -> Self {
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

                println!("{:#?}", names);
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
        call_expr: &CallExpr,
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
        // this is terminal, we don't need to traverse down
    }
}

struct DebugIdInjectVisitor {
    debug_id: Option<String>,
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
        if let Some(debug_id) = self.debug_id.take() {
            let used_export =
                get_relavant_call(call_expr, &self.namespace_import, &self.import_identifiers);

            if let Some(used_export) = used_export {
                if let Some(max_params) = DEBUGGABLE_FUNCTION_CONFIG.get(&used_export) {
                    if call_expr.args.len() < *max_params {
                        call_expr.args.push(ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Str(Str::from(debug_id)))),
                        })
                    }
                }
            }
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
