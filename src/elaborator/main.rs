
use std::{
  fs::{File, read_dir}, path::{PathBuf},
  ptr::addr_of_mut, io::Read, sync::atomic::{AtomicU16, Ordering, fence},};

use crate::{
  detached, build_capture_tuple,
  build_destructor_tuple, mk_args_intro, mk_args_rec, mk_ty_intro, mk_ty_rec,
  support_structures::no_bullshit_closure::DetachedClosure,
  expression_trees::raw_syntax_nodes::{
    DeclPtr, DeclKind, Definition, ExprPtr, RawKind},
    };

use super::{
  action_chain::{ActionLink, TaskHandle, DataFrameSize,},
  environment::{PasteboardTable, DefaultTableStreamingIterator},
  diagnostics::DiagnosticService, presense_tester::PresenseSet,};


struct MainFrame {
  symbol_table: PasteboardTable<u64, DeclPtr>,
  diagnostics_engine: DiagnosticService,
  observant_dir_loc: PathBuf,
  counter: AtomicU16,
  item_set: PresenseSet<u64>,
}

