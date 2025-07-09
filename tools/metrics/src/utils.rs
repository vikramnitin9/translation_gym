use rustc_hir::{HirId, Node, ConstContext};
use rustc_hir::def_id::DefId;
use rustc_span::{Span, FileNameDisplayPreference, Pos};
use rustc_middle::ty::TyCtxt;
use rustc_middle::mir::{Local, Body, BasicBlock, BasicBlockData, Statement, StatementKind, VarDebugInfoContents};

use rustc_span::source_map::SourceMap;
use rustc_span::{SpanSnippetError, BytePos};

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

pub fn print_span<'tcx>(tcx: TyCtxt<'tcx>, span: &Span) {
    let source_map = tcx.sess.source_map();
    println!(
        "{}\n```rust\n{}\n```\n",
        source_map.span_to_diagnostic_string(span.clone()),
        source_map.span_to_snippet(span.clone()).unwrap()
    );
}

pub fn span_to_string<'tcx>(tcx: TyCtxt<'tcx>, span: &Span) -> String {
    let source_map = tcx.sess.source_map();
    source_map.span_to_snippet(span.clone()).unwrap()
}

pub fn span_to_diagnostic<'tcx>(tcx: TyCtxt<'tcx>, span: &Span) -> String {
    let source_map = tcx.sess.source_map();
    source_map.span_to_diagnostic_string(span.clone())
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

pub fn get_mir_fn_from_defid<'tcx>(
    tcx: &TyCtxt<'tcx>,
    def_id: DefId,
) -> Option<&'tcx Body<'tcx>> {

    if tcx.is_mir_available(def_id)
        && matches!(
            tcx.hir().body_const_context(def_id.expect_local()),
            None | Some(ConstContext::ConstFn)
        )
    {
        Some(tcx.optimized_mir(def_id))
    } else {
        // debug!(
        //     "Skipping an item {:?}, no MIR available for this item",
        //     def_id
        // );
        None
    }
}

pub fn get_basic_block_span(block: &BasicBlockData<'_>, with_terminator: bool) -> Option<Span> {
    let mut aggregated_span: Option<Span> = None;

    for statement in block.statements.iter() {
        if let Some(mut aggregated_span) = aggregated_span {
            aggregated_span = aggregated_span.to(statement.source_info.span);
        }
        else {
            aggregated_span = Some(statement.source_info.span);
        }
    }
    if with_terminator {
        if let Some(terminator) = &block.terminator {
            if let Some(mut aggregated_span) = aggregated_span {
                aggregated_span = aggregated_span.to(terminator.source_info.span);
            }
            else {
                aggregated_span = Some(terminator.source_info.span);
            }
        }
    }
    aggregated_span
}

pub fn get_span_lines<'tcx>(tcx: TyCtxt<'tcx>, span: &Span) -> usize {
    let ((_, start_line, _), (_, end_line, _)) = span_to_data(tcx, span);
    end_line - start_line + 1
}

pub fn get_span_length<'tcx>(tcx: TyCtxt<'tcx>, span: &Span) -> u32 {
    let source_map = tcx.sess.source_map();
    let lo = source_map.lookup_byte_offset(span.lo());
    let hi = source_map.lookup_byte_offset(span.hi());
    hi.pos.0 - lo.pos.0
}

// Shrink the span from the end until the predicate returns false
pub fn span_trim_from_end_to_char(
    source_map: &SourceMap,
    span: Span,
    f: impl Fn(char) -> bool,
) -> Result<Span, SpanSnippetError> {
    let snippet = source_map.span_to_snippet(span)?;
    let n = snippet.chars()
                        .rev()
                        .collect::<String>()
                        .char_indices()
                        .find(|&(_, c)| !f(c)).map_or(0, |(i, _)| i);
    Ok(span.with_hi(span.hi() - BytePos(n as u32)))
}

pub fn span_trim_first_char(
    source_map: &SourceMap,
    span: Span,
) -> Result<Span, SpanSnippetError> {
    let snippet = source_map.span_to_snippet(span)?;
    Ok(span.with_lo(span.lo() + BytePos(1 as u32)))
}

// Shrink the span from the start until the predicate returns false
pub fn span_trim_from_start_to_char(
    source_map: &SourceMap,
    span: Span,
    f: impl Fn(char) -> bool,
) -> Result<Span, SpanSnippetError> {
    let snippet = source_map.span_to_snippet(span)?;
    let n = snippet.char_indices()
                        .find(|&(_, c)| !f(c)).map_or(0, |(i, _)| i);
    Ok(span.with_lo(span.lo() + BytePos(n as u32)))
}

// Expand the span backwards from the start till the predicate returns false
pub fn span_extend_from_start_to_char(
    source_map: &SourceMap,
    sp: Span,
    f: impl Fn(char) -> bool,
    accept_newlines: bool
) -> Span {
    if let Ok(prev_source) = source_map.span_to_prev_source(sp) {
        let prev_source = prev_source.rsplit(|c| !f(c)).next().unwrap_or("");
        if !prev_source.is_empty() && (accept_newlines || !prev_source.contains('\n')) {
            return sp.with_lo(BytePos(sp.lo().0 - prev_source.len() as u32));
        }
    }
    sp
}

pub fn get_first_bb_after_start_of_span<'tcx>(span: &Span, body: &'tcx Body<'tcx>) -> Option<BasicBlock> {
    // There is no guarantee that the blocks are in order of the source code
    // So we need to iterate through the blocks to find all the blocks that start before the span
    // and then return the one that is closest to the start of the span
    let start_pos = span.lo();
    let mut first_bb: Option<BasicBlock> = None;
    let mut min_dist: Option<u32> = None;

    for (bb, bb_data) in body.basic_blocks.iter_enumerated() {
        let bb_span = get_basic_block_span(bb_data, true);
        if bb_span.is_none() {
            continue;
        }
        let bb_start_pos = bb_span.unwrap().lo();
        if bb_start_pos.0 > start_pos.0  {
            let dist: u32 = bb_start_pos.0 - start_pos.0;
            if let Some(min) = min_dist {
                if dist < min {
                    first_bb = Some(bb);
                    min_dist = Some(dist);
                }
            }
            else {
                first_bb = Some(bb);
                min_dist = Some(dist);
            }
        }
    }
    first_bb
}

pub fn get_last_bb_before_start_of_span<'tcx>(span: &Span, body: &'tcx Body<'tcx>) -> Option<BasicBlock> {
    // There is no guarantee that the blocks are in order of the source code
    // So we need to iterate through the blocks to find all the blocks that start before the span
    // and then return the one that is closest to the start of the span
    let start_pos = span.lo();
    let mut first_bb: Option<BasicBlock> = None;
    let mut min_dist: Option<u32> = None;

    for (bb, bb_data) in body.basic_blocks.iter_enumerated() {
        let bb_span = get_basic_block_span(bb_data, true);
        if bb_span.is_none() {
            continue;
        }
        let bb_start_pos = bb_span.unwrap().lo();
        if start_pos.0 > bb_start_pos.0  {
            let dist: u32 = start_pos.0 - bb_start_pos.0;
            if let Some(min) = min_dist {
                if dist < min {
                    first_bb = Some(bb);
                    min_dist = Some(dist);
                }
            }
            else {
                first_bb = Some(bb);
                min_dist = Some(dist);
            }
        }
    }
    first_bb
}

pub fn get_last_bb_before_end_of_span(span: &Span, body: &Body<'_>) -> Option<BasicBlock> {
    // There is no guarantee that the blocks are in order of the source code
    // So we need to iterate through the blocks to find all the blocks that end after the span
    // and then return the one that is closest to the end of the span
    let end_pos = span.hi();
    let mut last_bb: Option<BasicBlock> = None;
    let mut min_dist: Option<u32> = None;

    for (bb, bb_data) in body.basic_blocks.iter_enumerated() {
        let bb_span = get_basic_block_span(bb_data, true);
        if bb_span.is_none() {
            continue;
        }
        let bb_end_pos = bb_span.unwrap().hi();
        if end_pos.0 > bb_end_pos.0 {
            let dist: u32 = end_pos.0 - bb_end_pos.0;
            if let Some(min) = min_dist {
                if dist < min {
                    last_bb = Some(bb);
                    min_dist = Some(dist);
                }
            }
            else {
                last_bb = Some(bb);
                min_dist = Some(dist);
            }
        }
    }
    last_bb

}

pub fn get_first_bb_after_end_of_span(span: &Span, body: &Body<'_>) -> Option<BasicBlock> {
    // There is no guarantee that the blocks are in order of the source code
    // So we need to iterate through the blocks to find all the blocks that end after the span
    // and then return the one that is closest to the end of the span
    let end_pos = span.hi();
    let mut last_bb: Option<BasicBlock> = None;
    let mut min_dist: Option<u32> = None;

    for (bb, bb_data) in body.basic_blocks.iter_enumerated() {
        let bb_span = get_basic_block_span(bb_data, true);
        if bb_span.is_none() {
            continue;
        }
        let bb_end_pos = bb_span.unwrap().hi();
        if bb_end_pos.0 > end_pos.0 {
            let dist: u32 = bb_end_pos.0 - end_pos.0;
            if let Some(min) = min_dist {
                if dist < min {
                    last_bb = Some(bb);
                    min_dist = Some(dist);
                }
            }
            else {
                last_bb = Some(bb);
                min_dist = Some(dist);
            }
        }
    }
    last_bb

}

pub fn select_statements_within_span<'tcx>(block: BasicBlock, span: &Span, body: &'tcx Body<'tcx>)
    -> Vec<&'tcx Statement<'tcx>>
{
    let mut selected_statements: Vec<&Statement<'_>> = Vec::new();
    let block_data = &body.basic_blocks[block];
    for statement in block_data.statements.iter() {
        if statement.source_info.span.overlaps(*span) {
            selected_statements.push(statement)
        }
    }
    selected_statements
}

pub fn get_live_dead_from_statements<'tcx>(statements: Vec<&'tcx Statement<'tcx>>)
    -> (Vec<Local>, Vec<Local>)
{
    let mut live: Vec<Local> = Vec::new();
    let mut dead: Vec<Local> = Vec::new();
    for statement in statements {
        match statement.kind {
            StatementKind::StorageLive(local) => live.push(local),
            StatementKind::StorageDead(local) => dead.push(local),
            _ => continue,
        }
    }
    (live, dead)
}

pub fn get_aggregated_span<'tcx>(block1_data: &BasicBlockData<'_>, block2_data: &BasicBlockData<'_>, tcx: TyCtxt<'tcx>) -> Option<Span> {

    let source_map = tcx.sess.source_map();

    let span1 = get_basic_block_span(block1_data, false);
    let span2 = get_basic_block_span(block2_data, true);
    if span1.is_none() || span2.is_none() {
        return None;
    }
    let mut aggregated_span = span2.unwrap().to(span1.unwrap());
    // Shrink the span to the last semicolon or closing brace before the end of the span
    if let Ok(new_span) = span_trim_from_end_to_char(source_map, aggregated_span, |c| {c != ';' && c != '}'}) {
        aggregated_span = new_span;
    }
    // Expand the span to just after the previous semicolon or closing brace before the start of the span
    if let Ok(new_span) = span_trim_from_start_to_char(source_map, aggregated_span, |c| {!";{}".contains(c)}) {
        aggregated_span = new_span;
    }
    // Chop off the first character because it is a semicolon or closing brace
    if let Ok(new_span) = span_trim_first_char(source_map, aggregated_span) {
        aggregated_span = new_span;
    }
    // If the span begins with newlines, cut them off
    if let Ok(new_span) = span_trim_from_start_to_char(source_map, aggregated_span, |c| {c == '\n'}) {
        aggregated_span = new_span;
    }
    Some(aggregated_span)
}

pub fn local_to_symbol<'tcx>(local: Local, body: &'tcx Body<'tcx>) -> Option<String> {
    let mut symbol: Option<String> = None;
    let var_debug_infos = &body.var_debug_info;
    for debug_info in var_debug_infos.iter().cloned() {
        if let VarDebugInfoContents::Place(pl) = debug_info.value {
            if pl.local == local {
                symbol = Some(debug_info.name.to_string());
                break;
            }
        }
    }
    symbol
}

pub fn print_locals<'tcx>(locals: Vec<Local>, body: &'tcx Body<'tcx>) {
    // Map each one to symbol and print only the ones which have symbols
    let symbols: Vec<String> = locals.iter()
                        .map(|l| local_to_symbol(*l, body))
                        .filter(|s| s.is_some())
                        .map(|s| s.unwrap())
                        .collect();
    println!("{:?}", symbols);
}