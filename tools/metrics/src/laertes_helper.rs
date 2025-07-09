use std::collections::{HashMap, HashSet};
use DataFlowNode::*;

pub type FnName = String;

#[derive(Debug)]
pub enum DataFlowNode {
    /// Return value of an intrinsic function
    RetVal,
    /// Parameter of an intrinsic function
    Param(usize),
    /// Points-to set of another node
    PtsTo(Box<DataFlowNode>),
}

impl DataFlowNode {
    pub fn pts_to(pointer: DataFlowNode) -> DataFlowNode {
        PtsTo(Box::new(pointer))
    }
}

pub type DataFlow = (DataFlowNode, DataFlowNode);

/// Helper functions we generate
pub static LAERTES_HELPERS: [&'static str; 11] = [
    "::borrow",
    "::borrow_mut",
    "::owned_as_ref",
    "::owned_as_mut",
    "::option_to_raw",
    "::_ref_eq",
    "::_ref_ne",
    "::trans",
    "::_as_mut_ptr",
    "::_as_ptr",
    "::_move_to_ptr",
];

/// Is this a special fn that is added by us or C2Rust before
/// create-initial-program
pub fn is_special_fn_in_benchmark(fn_name: &str) -> bool {
    fn_name.ends_with("::main")
        || fn_name.contains("bitfields::")
        || fn_name.ends_with("xmlschemastypes::get_bits")
        || fn_name.ends_with("xmlschemastypes::set_bits")
        || fn_name.contains("xmlschemastypes::_xmlSchemaValDate::")
        || fn_name.contains("xmlschemastypes::_xmlSchemaValDecimal::")
}

pub fn is_laertes_helper(fn_name: &str) -> bool {
    fn_name.contains("__laertes_array::")
        || fn_name.contains("laertes_rt::")
        || LAERTES_HELPERS
            .iter()
            .any(|helper| fn_name.ends_with(helper))
}
