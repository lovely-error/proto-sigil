
use std::{
  fs, path::PathBuf,
  ptr::addr_of_mut,
};

use crate::{
  detached,
  support_structures::no_bullshit_closure::DetachedClosure,
  expression_trees::{
    raw_syntax_nodes::{
      DeclPtr, DeclKind, Definition, ExprPtr, RawKind
    },
    better_nodes::Symbol
  },
};

use super::{
  action_chain::{ActionLink, TaskContext,},
  environment::{PasteboardTable, DefaultTableStreamingIterator},
  diagnostics::DiagnosticService, presense_tester::PresenseSet,
};


struct EnvBuildState {
  symbol_table: PasteboardTable<Symbol, DeclPtr>,
  diagnostics_engine: DiagnosticService,
  observant_dir_loc: PathBuf,
  symbol_interner: (),
}


pub fn elab_invocation_setup(root_folder_path: PathBuf) -> ActionLink {

  let start = ActionLink::make_gateway(detached!([root_folder_path] |ctx: TaskContext| {
    let env = ctx.interpret_frame::<EnvBuildState>();
    unsafe { addr_of_mut!(env.observant_dir_loc).write(root_folder_path) };
    return ActionLink::from_fun(begin_processing_files);
  }).erase_to_sendable());
  return ActionLink::make_autosized_frame_request::<EnvBuildState>(start);
}

fn begin_processing_files(ctx: TaskContext) -> ActionLink {

  let EnvBuildState {
    symbol_table,
    diagnostics_engine,
    observant_dir_loc,
    symbol_interner
  } = ctx.interpret_frame::<EnvBuildState>();

  let root_folder =
    fs::read_dir(observant_dir_loc);

  match root_folder {
    Ok(val) => {
      println!("Openned successfully {:#?}", val)
    },
    Err(err) => {
      println!("{err}")
    },
  }

  return ActionLink::make_completion();
}