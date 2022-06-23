use std::path::PathBuf;

use proto_sigil::elaborator::{worker::WorkGroupRef, main::build_parsing_task};


fn main () {
  let path = "/Users/cromobeingnur/testim_sigi";
  let path = PathBuf::from(path);

  let task = build_parsing_task(path);

  let exec =
    WorkGroupRef::init(1, task);
  exec.await_completion();
}