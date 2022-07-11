

use std::{sync::Mutex, collections::HashSet,};
use crate::expression_trees::{raw_syntax_nodes::SourceLocation, better_nodes::Symbol};

use super::frame_allocator::{RcTaskBoxRef};



pub struct DiagnosticService {
  reports: Mutex<Vec<ProblemReport>>
    // nobody is going to write code that consist mostly of error, right?
}
impl DiagnosticService {
  pub fn init() -> Self {
    Self { reports: Mutex::new(Vec::new()) }
  }
  pub fn report_problem(&self, problem: ProblemReport) {
    let mut item =
      self.reports.lock().unwrap();
    item.push(problem);
    drop(item);
  }
  pub fn did_record_any_issues(&self) -> bool {
    let item = self.reports.lock().unwrap();
    let count = item.len();
    drop(item);
    return count != 0
  }
  pub fn dump_reports_to_stdout(&self) {
    let reps = self.reports.lock().unwrap();
    let len = reps.len();
    println!("Encountered {} error{}.", len, if len > 1 {"s"} else {""} );
    // todo
  }
}

pub struct DiagnosticsDelegate {
  pub reports: Vec<ProblemReport>,
  pub associated_text: RcTaskBoxRef<String>,
}
impl DiagnosticsDelegate {
  pub fn report_problem(&mut self, report: ProblemReport) {
    self.reports.push(report)
  }
}

pub trait SomeDiagnosticsDelegate {
  fn report_problem(&mut self, report: ProblemReport);
}

impl SomeDiagnosticsDelegate for DiagnosticsDelegate {
  fn report_problem(&mut self, report: ProblemReport) {
    self.report_problem(report)
  }
}

#[derive(Debug, Clone,)]
pub struct ProblemReport {
  pub kind: Kind,
}


#[derive(Debug, Clone,)]
pub enum Kind {
  DuplicateDecls {
    one: Symbol,
    another: Symbol
  },
  IrrelevantSymbol(Symbol),
  DuplicatedBinders(HashSet<Symbol>),
  InvalidDeconstructionPattern(Symbol),
  IncorrectArity(SourceLocation),
  DuplicatesInImpCtx(HashSet<Symbol>),
  UnsedImpCtxAtTerminalNode(SourceLocation),
  UnusedItemsInImpCtx(HashSet<Symbol>),
  MismatchedType {
    type_expr: SourceLocation,
    term_expr: SourceLocation
  },
  ArityMismatch {
    expected: usize,
    found: usize,
    clause_loc: SourceLocation
  },
  BinderShapeConflict {
    pattern_loc: SourceLocation,
  },
  NonfuncTypeInFuncPos(SourceLocation)
}

