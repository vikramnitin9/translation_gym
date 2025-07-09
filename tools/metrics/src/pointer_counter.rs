use rustc_hir::{self, intravisit};
use rustc_middle::ty::{TyCtxt, TypeckResults};
use rustc_middle::hir::nested_filter;
use rustc_span::Span;

use crate::utils::*;

macro_rules! skip_generated_code {
    ($span: expr) => {
        if $span.from_expansion() || $span.is_dummy() {
            return;
        }
    };
}

pub struct PointerCounter<'tcx> {
    tcx: TyCtxt<'tcx>,
    current_typeck: Option<&'tcx TypeckResults<'tcx>>,
    pub all_derefs: Vec<Span>,
    pub all_decls: Vec<Span>,
    pub selected_fns: Vec<String>,
}

impl<'tcx> PointerCounter<'tcx> {
    pub fn new(tcx: &TyCtxt<'tcx>, selected_fns: Vec<String>) -> PointerCounter<'tcx> {
        PointerCounter{
            tcx: *tcx,
            current_typeck: None,
            all_derefs: Vec::new(),
            all_decls: Vec::new(),
            selected_fns,
        }
    }
}

impl<'tcx> intravisit::Visitor<'tcx> for PointerCounter<'tcx> {

    type NestedFilter = nested_filter::OnlyBodies;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.tcx.hir()
    }
    
    fn visit_item(&mut self, item: &'tcx rustc_hir::Item) {
        
        if let rustc_hir::ItemKind::Fn(..) = item.kind {
            // Check if name is in the selected functions
            if !self.selected_fns.is_empty() && !self.selected_fns.contains(&item.ident.name.as_str().to_string()) {
                return;
            }
        }
        // skip_generated_code!(item.span);
        let old_typeck = self.current_typeck;
        if self.tcx.has_typeck_results(item.owner_id.def_id) {
            self.current_typeck = Some(self.tcx.typeck(item.owner_id.def_id));
        }
        else {
            self.current_typeck = None;
        }
        intravisit::walk_item(self, item);
        self.current_typeck = old_typeck;
    }

    fn visit_expr(&mut self, expr: &'tcx rustc_hir::Expr) {
        // skip_generated_code!(expr.span);

        if let rustc_hir::ExprKind::Unary(rustc_hir::UnOp::Deref, sub_expr) = expr.kind {
            if let Some(typeck) = self.current_typeck {
                if typeck.expr_ty(sub_expr).is_unsafe_ptr() {
                    self.all_derefs.push(expr.span);
                }
            }
            // if self.current_typeck.is_none() {
            //     println!("Self.current_typeck is None");
            // }
        }
        intravisit::walk_expr(self, expr);
    }
    fn visit_pat(&mut self, pat: &'tcx rustc_hir::Pat) {
        // skip_generated_code!(pat.span);
        
        pat.walk_always(|p| {
            if let rustc_hir::PatKind::Binding(_, _, _, _) = p.kind {
                if let Some(typeck) = self.current_typeck {
                    let ty = typeck.pat_ty(pat);
                    if ty.is_unsafe_ptr() {
                        self.all_decls.push(pat.span);
                    }
                }
            }
        });
        intravisit::walk_pat(self, pat);
    }
}