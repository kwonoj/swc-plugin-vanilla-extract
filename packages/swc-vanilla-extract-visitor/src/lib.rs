use swc_core::{
    common::{comments::Comments, SourceMapper},
    ecma::visit::{noop_visit_mut_type, VisitMut},
};

pub struct VanillaExtractVisitor {}

impl VisitMut for VanillaExtractVisitor {
    noop_visit_mut_type!();
}

pub fn create_extract_visitor<C: Clone + Comments, S: SourceMapper>(
    _source_map: std::sync::Arc<S>,
    _comments: C,
    _filename: String,
) -> VanillaExtractVisitor {
    VanillaExtractVisitor {}
}
