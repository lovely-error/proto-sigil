

use std::{sync::Mutex,};
use crate::parser::parser::SourceLocation;
use super::frame_allocator::{RcTaskBoxRef};



pub struct DiagnosticEngine {
  reports: Mutex<Vec<ProblemReport>>
    // nobody is going to write code that consist mostly of error, right?
}
impl DiagnosticEngine {
  pub fn init() -> Self {
    Self { reports: Mutex::new(Vec::new()) }
  }
  pub fn report_problem(&self, problem: ProblemReport) {
    let mut item =
      self.reports.lock().unwrap();
    item.push(problem);
    drop(item);
  }
  pub fn recorded_any_issues(&self) -> bool {
    let item = self.reports.lock().unwrap();
    let count = item.len();
    drop(item);
    return count != 0
  }
  pub fn dump_reports_to_stdout(&self) {
    let reps = self.reports.lock().unwrap();
    let len = reps.len();
    println!("Encountered {} error{}.", len, if len > 1 {"s"} else {""} );
    for rep in reps.iter() {
      let rep = rep.render();
      println!("{}", rep)
    }
  }
}

#[derive(Debug, Clone,)]
pub struct ProblemReport {
  pub kind: Kind,
  pub associated_text: RcTaskBoxRef<String>
}
impl ProblemReport {
  pub fn render(&self) -> String {
    let str = self.associated_text.get_ref();

    match self.kind {
      Kind::Duplicates(rep) => {
        fn render_thing(
          err_msg_buf: &mut String, loc: SourceLocation, str: &str) {
          let start = loc.primary_offset as usize;
          let pre_span = &str[..start];
          let mut count = 1u32;
          for i in pre_span.chars() {
            if i == '\n' { count += 1 }
          }
          let span =
            start .. loc.secondary_offset as usize;
          let view = &str[span];
          *err_msg_buf += format!(
            "Definition {} at line {} is duplicated.",
            view, count).as_str();
        }

        let mut err_msg = String::new();
        render_thing(&mut err_msg, rep.one, str);
        err_msg.push('\n');
        render_thing(&mut err_msg, rep.another, str);
        err_msg.push('\n');
        return err_msg
      },
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
  Duplicates(DuplicateObjects)
}

impl DiagnosticEngine {
  pub fn report_about_duplicate
  (&self, report: DuplicateObjects, str_ref: RcTaskBoxRef<String>) {
    let report =
      ProblemReport {
        kind: Kind::Duplicates(report), associated_text: str_ref };
    let mut lock =
      self.reports.lock().unwrap();
    lock.push(report);
    drop(lock);
  }
}

#[derive(Debug, Clone, Copy)]
pub struct DuplicateObjects {
  pub one: SourceLocation,
  pub another: SourceLocation
}