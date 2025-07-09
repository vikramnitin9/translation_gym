use rustc_hir::{self, intravisit, Block};
use rustc_middle::ty::TyCtxt;
use rustc_middle::hir::nested_filter;
use rustc_span::Span;

macro_rules! skip_generated_code {
    ($span: expr) => {
        if $span.from_expansion() || $span.is_dummy() {
            return;
        }
    };
}

pub struct UnsafeCounter<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub unsafe_spans: Vec<Span>,
    pub unsafe_calls: Vec<Span>,
    pub unsafe_casts: Vec<Span>,
    pub inside_unsafe: bool,
    pub selected_fns: Vec<String>,
}

impl<'tcx> UnsafeCounter<'tcx> {
    pub fn new(tcx: &TyCtxt<'tcx>, selected_fns: Vec<String>) -> UnsafeCounter<'tcx> {
        UnsafeCounter{
            tcx: *tcx,
            unsafe_spans: Vec::new(),
            unsafe_calls: Vec::new(),
            unsafe_casts: Vec::new(),
            inside_unsafe: false,
            selected_fns,
        }
    }
}

impl<'tcx> intravisit::Visitor<'tcx> for UnsafeCounter<'tcx> {

    type NestedFilter = nested_filter::OnlyBodies;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.tcx.hir()
    }
    
    fn visit_item(&mut self, item: &'tcx rustc_hir::Item) {
        // skip_generated_code!(item.span);
        // Visit the item
        match &item.kind {
            rustc_hir::ItemKind::Fn(fn_sig, _, _) => {
                // Check if name is in the selected functions
                if !self.selected_fns.is_empty() && !self.selected_fns.contains(&item.ident.name.as_str().to_string()) {
                    return;
                }
                if fn_sig.header.safety == rustc_hir::Safety::Unsafe {
                    // Add the entire function to the list of unsafe spans
                    self.unsafe_spans.push(item.span);

                    let old_inside_unsafe = self.inside_unsafe;
                    self.inside_unsafe = true;
                    intravisit::walk_item(self, item);
                    self.inside_unsafe = old_inside_unsafe;
                }
                else {
                    // Visit the item
                    intravisit::walk_item(self, item);
                }
            },
            _ => {
                // Visit the item
                intravisit::walk_item(self, item);
            }
        }
    }
    fn visit_block(&mut self, block: &'tcx Block) {
        // skip_generated_code!(block.span);
        
        if block.rules == rustc_hir::BlockCheckMode::UnsafeBlock(rustc_hir::UnsafeSource::UserProvided) {
            // Add the entire block to the list of unsafe spans
            self.unsafe_spans.push(block.span);

            let old_inside_unsafe = self.inside_unsafe;
            self.inside_unsafe = true;
            intravisit::walk_block(self, block);
            self.inside_unsafe = old_inside_unsafe;
        }
        else {
            // Visit the block
            intravisit::walk_block(self, block);
        }
    }
    fn visit_expr(&mut self, expr: &'tcx rustc_hir::Expr) {
        // skip_generated_code!(expr.span);
        
        if self.inside_unsafe {
            match &expr.kind {
                rustc_hir::ExprKind::Call(..) =>  {
                    // Add the call to the list of unsafe calls
                    self.unsafe_calls.push(expr.span);
                },
                rustc_hir::ExprKind::MethodCall(..) => {
                    // Add the call to the list of unsafe calls
                    self.unsafe_calls.push(expr.span);
                },
                rustc_hir::ExprKind::Cast(..) => {
                    // Add the cast to the list of unsafe casts
                    self.unsafe_casts.push(expr.span);
                },
                _ => {}
            }
        }
        intravisit::walk_expr(self, expr);
    }
}