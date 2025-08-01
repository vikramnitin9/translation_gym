use rustc_hir::{HirId, Node};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;
use rustc_middle::ty::ParamEnvAnd;
use std::collections::{HashMap, HashSet};
use rustc_hir::intravisit;
use rustc_middle::hir::nested_filter;
use rustc_span::Span;
use std::path::PathBuf;

use crate::utils::*;

macro_rules! skip_generated_code {
    ($span: expr) => {
        if $span.from_expansion() || $span.is_dummy() {
            return;
        }
    };
}

// Backup self.cur_fn, set cur_fn to id, continue to walk the AST by executing
// $walk, then restore self.cur_fn.
macro_rules! push_walk_pop {
    ($this: expr, $id: expr, $walk: expr) => {{
        let prev_fn = $this.cur_fn;
        $this.cur_fn = Some($id);
        $walk;
        $this.cur_fn = prev_fn;
    }};
}

#[derive(Hash, PartialEq, Eq, Debug)]
pub struct Call {
    // the call expression
    pub call_expr: HirId,
    pub call_expr_span: Span,
    // possible enclosing function
    pub caller: Option<DefId>,
    pub caller_span: Option<Span>,
    // call target
    pub callee: DefId,
    pub callee_span: Span,
}

pub struct FnInfo {
    pub span: Span,
    pub sig_span: Span,
    pub foreign: bool,
    pub globals: HashMap<DefId, Span>,
    pub structs: HashMap<DefId, Span>,
    pub enums: HashMap<DefId, Span>,
    pub impl_id: Option<DefId>, // the impl id if this is a method
}

pub struct CallgraphVisitor<'tcx> {
    // type context
    pub tcx: TyCtxt<'tcx>,

    // free functions
    pub functions: HashMap<DefId, FnInfo>,
    // global variables
    pub globals: HashMap<DefId, Span>,
    // structs
    pub structs: HashMap<DefId, Span>,
    // enums
    pub enums: HashMap<DefId, Span>,
    // trait method declarations without default implementation
    pub method_decls: HashSet<DefId>,
    // map decls to impls
    pub method_impls: HashMap<DefId, Vec<DefId>>,

    // static calls
    pub static_calls: HashSet<Call>,
    // dynamic calls
    pub dynamic_calls: HashSet<Call>,

    pub imports: HashSet<Span>,
    // all source files in the crate
    pub files: HashSet<PathBuf>,

    // tracks the current function we're in during AST walk
    cur_fn: Option<DefId>,
}

impl<'tcx> CallgraphVisitor<'tcx> {
    pub fn new(tcx: &TyCtxt<'tcx>) -> CallgraphVisitor<'tcx> {
        CallgraphVisitor {
            tcx: *tcx,
            functions: HashMap::new(),
            globals: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            method_decls: HashSet::new(),
            method_impls: HashMap::new(),
            static_calls: HashSet::new(),
            dynamic_calls: HashSet::new(),
            imports: HashSet::new(),
            files: HashSet::new(),
            cur_fn: None,
        }
    }
}

impl<'tcx> intravisit::Visitor<'tcx> for CallgraphVisitor<'tcx> {

    type NestedFilter = nested_filter::OnlyBodies;

    fn nested_visit_map(&mut self) -> Self::Map {
        self.tcx.hir()
    }

    fn visit_expr(&mut self, expr: &'tcx rustc_hir::Expr) {
        // Don't do this - it skips macros like println!
        // skip_generated_code!(expr.span);

        let hir_id = expr.hir_id;
        match expr.kind {
            rustc_hir::ExprKind::Call(
                    rustc_hir::Expr{
                        kind: rustc_hir::ExprKind::Path(ref qpath),
                        hir_id: sub_hir_id,
                        ..
                    }, _) => {
                if let rustc_hir::QPath::Resolved(_, p) = qpath {
                    if let rustc_hir::def::Res::Def(_, def_id) = p.res {
                        self.static_calls.insert(Call {
                            call_expr: hir_id,
                            call_expr_span: expr.span,
                            caller: self.cur_fn,
                            caller_span: None,
                            callee: def_id,
                            callee_span: p.span,
                        });
                    }
                }
                // Functions like ::new and ::default apparently need to be resolved using type checking
                let o_def_id = hir_id.owner;
                let typeck_tables = self.tcx.typeck(o_def_id);
                let substs = typeck_tables.node_args(hir_id);
                let method_id = typeck_tables.type_dependent_def_id(*sub_hir_id);
                if let Some(method_id) = method_id {
                    self.static_calls.insert(Call {
                        call_expr: hir_id,
                        call_expr_span: expr.span,
                        caller: self.cur_fn,
                        caller_span: None,
                        callee: method_id,
                        callee_span: Span::default(), // TODO: get the span of the method call
                    });
                }
            },            
            rustc_hir::ExprKind::MethodCall(_, _, _, _) => {
                let o_def_id = hir_id.owner;
                let typeck_tables = self.tcx.typeck(o_def_id);
                // let substs = typeck_tables.node_substs(hir_id); // Substitutions
                let substs = typeck_tables.node_args(hir_id); // For nightly-2024-08-07
                let method_id = typeck_tables.type_dependent_def_id(hir_id).expect("fail");
                // TODO: find out whether this is all needed or whether we can directly use the method_id
                let param_env = self.tcx.param_env(method_id);
                if let Ok(Some(inst)) =
                    // self.tcx.resolve_instance(ParamEnvAnd{param_env, value: (method_id, substs)})
                    self.tcx.resolve_instance_raw(ParamEnvAnd{param_env, value: (method_id, substs)}) // For nightly-2024-08-07
                {
                    let res_def_id = inst.def_id();
                    let node = self.tcx.hir().get_if_local(res_def_id);
                    match node {
                        Some(Node::TraitItem(rustc_hir::TraitItem{span, ..})) => {
                            // dynamic calls resolve only to the trait method decl
                            self.dynamic_calls.insert(Call {
                                call_expr: hir_id,
                                call_expr_span: expr.span,
                                caller: self.cur_fn,
                                caller_span: None,
                                callee: res_def_id,
                                callee_span: *span,
                            });
                        }
                        Some(Node::ImplItem(rustc_hir::ImplItem{span, ..})) |
                                Some(Node::Item(rustc_hir::Item{span, ..})) |
                                Some(Node::ForeignItem(rustc_hir::ForeignItem{span, ..})) => {
                            // calls for which the receiver's type can be resolved
                            self.static_calls.insert(Call {
                                call_expr: hir_id,
                                call_expr_span: expr.span,
                                caller: self.cur_fn,
                                caller_span: None,
                                callee: res_def_id,
                                callee_span: *span,
                            });
                        },
                        None => (),
                        _ => todo!(),
                    };
                }
            },
            _ => {}
        }
        // traverse
        intravisit::walk_expr(self, expr);
    }

    fn visit_qpath(&mut self, qpath: &'tcx rustc_hir::QPath<'tcx>, id: HirId, _span: Span) {
        skip_generated_code!(qpath.span());

        let cur_fn_name: String = self.cur_fn.
                                and_then(|def_id| Some(self.tcx.item_name(def_id).to_string())).
                                unwrap_or_else(|| "<unknown>".to_string());
        
        if let rustc_hir::QPath::Resolved(_, path) = qpath {
            if let rustc_hir::def::Res::Def(def_kind, def_id) = path.res {
                if let Some(node) = self.tcx.hir().get_if_local(def_id) {
                    match node {
                        Node::Item(rustc_hir::Item{span, ..}) |
                                Node::ForeignItem(rustc_hir::ForeignItem{span, ..}) => {
                            match def_kind {
                                rustc_hir::def::DefKind::Struct |
                                rustc_hir::def::DefKind::Union |
                                rustc_hir::def::DefKind::Enum | 
                                rustc_hir::def::DefKind::ForeignTy => {
                                    self.structs.insert(def_id, *span);
                                    // Add it to the structs for this function
                                    if let Some(cur_fn_def_id) = self.cur_fn {
                                        if let Some(func) = self.functions.get_mut(&cur_fn_def_id) {
                                            func.structs.insert(def_id, *span);
                                        }
                                    }
                                },
                                rustc_hir::def::DefKind::Static{..} |
                                rustc_hir::def::DefKind::Const => {
                                    self.globals.insert(def_id, *span);
                                    // Add it to the globals for this function
                                    if let Some(cur_fn_def_id) = self.cur_fn {
                                        if let Some(func) = self.functions.get_mut(&cur_fn_def_id) {
                                            func.globals.insert(def_id, *span);
                                        }
                                    }
                                },
                                _ => {}
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
        // traverse
        intravisit::walk_qpath(self, qpath, id);
    }

    fn visit_item(&mut self, item: &'tcx rustc_hir::Item) {
        skip_generated_code!(item.span);

        let local_path = match self.tcx.sess.source_map().span_to_filename(item.span) {
            rustc_span::FileName::Real(filename) => {
                match filename {
                    rustc_span::RealFileName::LocalPath(path) => Some(path),
                    rustc_span::RealFileName::Remapped{local_path: Some(local_path), ..} => Some(local_path),
                    _ => None,
                }
            },
            _ => None,
        };
        if let Some(path) = local_path {
            self.files.insert(path);
        }
        let hir_id = item.hir_id();
        match item.kind {
            rustc_hir::ItemKind::Fn(fn_sig, _, _) => {
                let def_id = hir_id.owner.to_def_id();
                self.functions.insert(def_id, FnInfo{span: item.span,
                                                     sig_span: fn_sig.span,
                                                     foreign: false,
                                                     globals: HashMap::new(),
                                                     structs: HashMap::new(),
                                                     enums: HashMap::new(),
                                                     impl_id: None});
                push_walk_pop!(self, def_id, intravisit::walk_item(self, item));
                return;
            },
            rustc_hir::ItemKind::Use(..) => {
                self.imports.insert(item.span);
                return;
            },
            rustc_hir::ItemKind::Static(..) => {
                // static variables
                let def_id = hir_id.owner.to_def_id();
                self.globals.insert(def_id, item.span);
            },
            rustc_hir::ItemKind::Const(..) => {
                // const variables
                let def_id = hir_id.owner.to_def_id();
                self.globals.insert(def_id, item.span);
            },
            rustc_hir::ItemKind::Struct(..) => {
                // structs
                let def_id = hir_id.owner.to_def_id();
                self.structs.insert(def_id, item.span);
            },
            rustc_hir::ItemKind::Enum(..) => {
                // enums
                let def_id = hir_id.owner.to_def_id();
                self.enums.insert(def_id, item.span);
            },
            _ => {}
        }
        // traverse
        intravisit::walk_item(self, item)
    }

    fn visit_trait_item(&mut self, ti: &'tcx rustc_hir::TraitItem) {
        skip_generated_code!(ti.span); // TODO ?do we want this

        let hir_id = ti.hir_id();
        let def_id = hir_id.owner.to_def_id();

        match ti.kind {
            rustc_hir::TraitItemKind::Fn(_, rustc_hir::TraitFn::Required(_)) => {
                // a method declaration
                self.method_decls.insert(def_id);
                self.method_impls.insert(def_id, vec![]);
            }
            rustc_hir::TraitItemKind::Fn(fn_sig, rustc_hir::TraitFn::Provided(_)) => {
                // a method decl and def
                self.method_decls.insert(def_id);
                self.functions.insert(def_id, FnInfo{span: ti.span,
                                                     sig_span: fn_sig.span,
                                                     foreign: false,
                                                     globals: HashMap::new(),
                                                     structs: HashMap::new(),
                                                     enums: HashMap::new(),
                                                     impl_id: None});
                self.method_impls.entry(def_id).or_default().push(def_id);

                push_walk_pop!(self, def_id, intravisit::walk_trait_item(self, ti));
            }
            _ => {}
        }

        // traverse
        intravisit::walk_trait_item(self, ti)
    }

    fn visit_impl_item(&mut self, ii: &'tcx rustc_hir::ImplItem) {
        skip_generated_code!(ii.span);

        let hir_id = ii.hir_id();
        let def_id = hir_id.owner.to_def_id();

        if let rustc_hir::ImplItemKind::Fn(fn_sig, _) = ii.kind {
            // store link to decl
            let mut decl_id = None;
            if let Some(impl_id) = self.tcx.impl_of_method(def_id) {
                if let Some(Node::Item(item)) = self.tcx.hir().get_if_local(impl_id) {
                    if let rustc_hir::ItemKind::Impl(rustc_hir::Impl{self_ty, ..}) = item.kind {
                        if let rustc_hir::TyKind::Path(rustc_hir::QPath::Resolved(_, path)) = self_ty.kind {
                            if let rustc_hir::def::Res::Def(_, impl_def_id) = path.res {
                                decl_id = Some(impl_def_id);
                            }
                        }
                    }
                }
            }

            self.functions.insert(def_id, FnInfo{span: ii.span,
                                                 sig_span: fn_sig.span,
                                                 foreign: false,
                                                 globals: HashMap::new(),
                                                 structs: HashMap::new(),
                                                 enums: HashMap::new(),
                                                 impl_id: decl_id});

            push_walk_pop!(self, def_id, intravisit::walk_impl_item(self, ii));

            return;
        }

        // traverse
        intravisit::walk_impl_item(self, ii)
    }

    fn visit_foreign_item(&mut self, fi: &'tcx rustc_hir::ForeignItem) {
        skip_generated_code!(fi.span);

        let hir_id = fi.hir_id();
        let def_id = hir_id.owner.to_def_id();

        match fi.kind {
            rustc_hir::ForeignItemKind::Fn(..) => {
                // The signature is the same as the function because there is no body
                self.functions.insert(def_id, FnInfo{span: fi.span,
                                                     sig_span: fi.span,
                                                     foreign: true,
                                                     globals: HashMap::new(),
                                                     structs: HashMap::new(),
                                                     enums: HashMap::new(),
                                                     impl_id: None});
                push_walk_pop!(self, def_id, intravisit::walk_foreign_item(self, fi));
                return;
            }
            rustc_hir::ForeignItemKind::Static(..) => {
                // foreign static variables
                self.globals.insert(def_id, fi.span);
            },
            rustc_hir::ForeignItemKind::Type => {
                // foreign types
                self.structs.insert(def_id, fi.span);
            },
        }

        // traverse
        intravisit::walk_foreign_item(self, fi)
    }

}