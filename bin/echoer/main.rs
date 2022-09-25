use std::{process::Stdio, ptr::addr_of_mut, io::{Stdin, self, Stdout, Write}};

use proto_sigil::elaborator::{action_chain::{TaskContext, ActionLink}, worker::WorkGroup, frame_allocator::SlabSize};




fn main () {
  let wg = ActionLink::from_fun(setup);
  let memed = ActionLink::make_frame_request(SlabSize::Bytes256, wg);
  let exec = WorkGroup::init(memed);
  exec.await_completion();
}

struct Frame {
  stdio: Stdin,
  stdout: Stdout,
  msg: String,
}

fn setup (ctx : TaskContext) -> ActionLink {
  let frame = ctx.interpret_frame::<Frame>();
  unsafe {
    addr_of_mut!(frame.stdio).write(io::stdin());
    addr_of_mut!(frame.stdout).write(io::stdout());
    addr_of_mut!(frame.msg).write(String::new());
  };

  return ActionLink::from_fun(echo_loop);
}

fn echo_loop (ctx : TaskContext) -> ActionLink {
  let frame = ctx.interpret_frame::<Frame>();
  frame.msg.clear();
  let _ = frame.stdio.read_line(&mut frame.msg);
  if frame.msg == "STOP\r\n" {
    return ActionLink::make_completion()
  } else {
    let _ = frame.stdout.write(frame.msg.as_bytes());
    return ActionLink::from_fun(echo_loop);
  }
}