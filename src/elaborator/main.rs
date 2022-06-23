
use std::{fs::{File, read_dir}, path::{Path, PathBuf}, ptr::addr_of_mut};

use crate::{
  detached, build_capture_tuple,
  build_destructor_tuple, mk_args_intro, mk_args_rec, mk_ty_intro, mk_ty_rec,
  support_structures::no_bullshit_closure::DetachedClosure};

use super::action_chain::{ActionLink, TaskHandle, DataFrameSize,};


struct ParseFrame {
  observant_dir_loc: PathBuf
}

pub fn build_parsing_task(dir_path: PathBuf) -> ActionLink {

  let det_clos =
  detached!([dir_path] |tfh:TaskHandle| {
    let frame = tfh.interpret_frame::<ParseFrame>();
    unsafe { addr_of_mut!(frame.observant_dir_loc).write(dir_path) };
    return ActionLink::goto(check_path_validity);
  });
  let gw =
    ActionLink::make_gateway(det_clos.erase_to_sendable());
  let framed =
    ActionLink::make_frame_request(
      DataFrameSize::Bytes120, gw);
  return framed;
}
fn check_path_validity(handle: TaskHandle) -> ActionLink {
  let frame = handle.interpret_frame::<ParseFrame>();
  let dir = &frame.observant_dir_loc;
  if !dir.exists() || !dir.is_dir() {
    return ActionLink::goto(stop_building_with_error);
  } else {
    return ActionLink::goto(frame_setup);
  }
}
fn stop_building_with_error(handle: TaskHandle) -> ActionLink {

  let frame = handle.interpret_frame::<ParseFrame>();

  let path = frame.observant_dir_loc.display();
  println!("Failed to proceed with path {path}");

  return ActionLink::make_completion(true);
}

fn frame_setup(handle: TaskHandle) -> ActionLink {

  let frame =
    handle.interpret_frame::<ParseFrame>();

  println!("\nGoing to proceed with {}", frame.observant_dir_loc.display());

  let dir = read_dir(&frame.observant_dir_loc);
  if let Err(err) = dir {
    println!("{err}");
    return ActionLink::goto(stop_building_with_error);
  }
  let dir = dir.unwrap();
  for i in dir {
    if let Ok(entry) = i {
      println!("{:?}", entry.file_name().to_str());
    }
  }

  return ActionLink::make_completion(true);
}
