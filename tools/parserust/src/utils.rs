use std::collections::HashSet;

use rustc_hir::{HirId, Node};

use rustc_middle::ty::TyCtxt;

use rustc_span::{Span, FileNameDisplayPreference, Pos};

pub fn get_parent_span(hir_id: &HirId, tcx: &TyCtxt) -> Option<Span> {
    // Given a node corresponding to an Expr,
    // follow the chain up until you reach a Stmt or a Block
    // and return the span of that node.
    for parent_node in tcx.hir().parent_iter(*hir_id) {
        match parent_node {
            (_, rustc_hir::Node::Stmt(stmt)) => return Some(stmt.span),
            (_, rustc_hir::Node::Block(block)) => return Some(block.span),
            _ => continue,
        }
    }
    if let Node::Expr(expr) = tcx.hir_node(*hir_id) {
        return Some(expr.span);
    }
    None
}

pub fn span_to_string<'tcx>(tcx: TyCtxt<'tcx>, span: &Span) -> String {
    let source_map = tcx.sess.source_map();
    source_map.span_to_snippet(span.clone()).unwrap()
}

// Given a span, return all the import statements in the same file
pub fn get_imports_from_same_file<'tcx>(span: &Span, imports: &HashSet<Span>, tcx: &TyCtxt<'tcx>) -> Vec<Span> {
    let source_map = tcx.sess.source_map();
    imports.iter().filter(|&import|
    {
        source_map.lookup_char_pos(span.lo()).file.name == source_map.lookup_char_pos(import.lo()).file.name
    })
    .map(|span| *span)
    .collect()
}

pub fn span_to_data<'tcx>(tcx: TyCtxt<'tcx>, span: &Span) -> ((String, usize, usize), (String, usize, usize)){
    let source_map = tcx.sess.source_map();
    let low_pos = source_map.lookup_char_pos(span.lo());
    let high_pos = source_map.lookup_char_pos(span.hi());
    let low_fname = low_pos.file.name.display(FileNameDisplayPreference::Remapped).to_string();
    let high_fname = high_pos.file.name.display(FileNameDisplayPreference::Remapped).to_string();

    ((low_fname, low_pos.line, low_pos.col.to_usize() + 1),
     (high_fname, high_pos.line, high_pos.col.to_usize() + 1))
}