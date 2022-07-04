
use std::{
  fs::{File, read_dir}, path::{PathBuf},
  ptr::addr_of_mut, io::Read, sync::atomic::{AtomicU16, Ordering, fence},};

use crate::{
  detached, build_capture_tuple,
  build_destructor_tuple, mk_args_intro, mk_args_rec, mk_ty_intro, mk_ty_rec,
  support_structures::no_bullshit_closure::DetachedClosure,
  trees::raw_syntax_nodes::{DeclPtr, DeclKind, Definition, ExprPtr, RawKind}, parser::parser::{ParsingState,},
  elaborator::diagnostics::DuplicateObjects};

use super::{
  action_chain::{ActionLink, TaskHandle, DataFrameSize,},
  environment::{PasteboardTable, DefaultTableStreamingIterator},
  diagnostics::DiagnosticEngine, presense_tester::PresenseSet,};


struct MainFrame {
  symbol_table: PasteboardTable<u64, DeclPtr>,
  diagnostics_engine: DiagnosticEngine,
  observant_dir_loc: PathBuf,
  counter: AtomicU16,
  item_set: PresenseSet<u64>,
}

pub fn build_parsing_task(dir_path: PathBuf) -> ActionLink {

  let det_clos =
  detached!([dir_path] |tfh:TaskHandle| {
    let frame = tfh.interpret_frame::<MainFrame>();
    unsafe { addr_of_mut!(frame.observant_dir_loc).write(dir_path) };
    return ActionLink::goto(check_path_validity);
  });
  let gw =
    ActionLink::make_gateway(det_clos.erase_to_sendable());
  let framed =
    ActionLink::make_frame_request(
      DataFrameSize::Bytes248, gw);
  return framed;
}
fn check_path_validity(handle: TaskHandle) -> ActionLink {
  let frame = handle.interpret_frame::<MainFrame>();
  let dir = &frame.observant_dir_loc;
  if !dir.exists() || !dir.is_dir() {
    return ActionLink::goto(stop_building_with_error);
  } else {
    return ActionLink::goto(frame_setup);
  }
}
fn stop_building_with_error(handle: TaskHandle) -> ActionLink {

  let frame = handle.interpret_frame::<MainFrame>();

  let path = frame.observant_dir_loc.display();
  println!("Failed to proceed with path {path}");

  return ActionLink::make_completion(true);
}

fn frame_setup(handle: TaskHandle) -> ActionLink {

  let frame =
    handle.interpret_frame::<MainFrame>();

  println!("\nGoing to proceed with {}", frame.observant_dir_loc.display());

  let dir = read_dir(&frame.observant_dir_loc);
  if let Err(err) = dir {
    println!("{err}");
    return ActionLink::goto(stop_building_with_error);
  }
  let mut files = Vec::new();
  let dir = dir.unwrap();
  for i in dir {
    if let Ok(entry) = i {
      if let Some(ext) = entry.path().extension() {
        if ext != "sg" { continue }
      }
      let file = File::open(entry.path()).unwrap();
      files.push(file);
    }
  }
  if files.is_empty() {
    return ActionLink::goto(|_| {
      println!("No files to check");
      return ActionLink::make_completion(true);
    });
  }

  let file_count = files.len();
  let fresh_table = PasteboardTable::init();
  unsafe {
    addr_of_mut!(frame.symbol_table).write(fresh_table);
    addr_of_mut!(frame.counter).write(AtomicU16::new(file_count as u16));
    addr_of_mut!(frame.diagnostics_engine).write(DiagnosticEngine::init());
    addr_of_mut!(frame.item_set).write(PresenseSet::init());
  };

  for file in files {
    let subtask = ActionLink::make_task_local_closure(
      &handle, file, process_file);
    handle.assign_work_for_schedule(subtask);
  }

  return ActionLink::make_progress_checker(parsing_awaiter);

}

fn parsing_awaiter(handle: TaskHandle) -> Option<ActionLink> {
  let frame = handle.interpret_frame::<MainFrame>();

  if frame.counter.load(Ordering::Relaxed) != 0 {
    return None;
  };

  fence(Ordering::Acquire);
  return Some(ActionLink::goto(sanitise_defns));
}

fn process_file(env: *mut File, handle : TaskHandle) -> ActionLink {
  let frame = handle.interpret_frame::<MainFrame>();
  let mut env = unsafe { env.read() };

  // read bytes lazily ?
  let mut chars = String::new();
  let _ = env.read_to_string(&mut chars);
  let file = handle.spawn_box(chars);
  let bytes = file.get_ref().as_bytes();

  let mut parser = ParsingState::init(bytes);
  'parsing : loop {
    parser.skip_trivia();
    if parser.no_more_chars() { break; }
    let one_decl = parser.parse_decl();
    match one_decl {
      Ok(decl) => {
        let name = decl.project_name();
        let hash = name.hash;
        let was_there = frame.item_set.check_in(&hash);
        if was_there {
          let other_item =
            frame.symbol_table.retrieve_ref(&hash).unwrap();
          let sloc1 = other_item.project_name().sloc ;
          let sloc2 = decl.project_name().sloc;
          let file_ref = file.vend_ref();
          frame.diagnostics_engine.report_about_duplicate(
            DuplicateObjects { another: sloc1, one: sloc2 },
            file_ref);
        } else {
          frame.symbol_table.insert(&hash, decl);
        }
      },
      Err(err) => {
        // investigate problemm.
        // then put report to diagnostics engine
        println!("{:?}", err);
        break 'parsing;
      },
    }
  }

  fence(Ordering::Release);
  frame.counter.fetch_sub(1, Ordering::Relaxed);
  return ActionLink::make_completion(false);
}

fn sanitise_defns(handle : TaskHandle) -> ActionLink {
  let frame = handle.interpret_frame::<MainFrame>();

  if frame.diagnostics_engine.recorded_any_issues() {
    frame.diagnostics_engine.dump_reports_to_stdout();
    return ActionLink::make_completion(true);
  };

  frame.symbol_table.freeze();

  let iter =
    DefaultTableStreamingIterator::init(&frame.symbol_table);
  let mut ix = 0;

  for i in iter {
    ix += 1;
    let clos =
    ActionLink::make_task_local_closure(&handle, i, |i, handle| {
      let i = unsafe { i.read() };
      let frame = handle.interpret_frame::<SanWorkerFrame>();
      frame.decl_ptr = i;
      frame.counter.store(0, Ordering::Relaxed);
      return ActionLink::goto(sanitise_obj);
    });
    let inj_mem =
      ActionLink::make_frame_request(
        DataFrameSize::Bytes56, clos);
    handle.assign_work_for_schedule(inj_mem);
  }

  frame.counter.store(ix, Ordering::Relaxed);

  return ActionLink::make_progress_checker(sanitation_awaiter);
}

fn sanitation_awaiter(handle : TaskHandle) -> Option<ActionLink> {
  let frame = handle.interpret_frame::<MainFrame>();
  if frame.counter.load(Ordering::Relaxed) != 0 {
    return None;
  }
  fence(Ordering::Acquire);
  return Some(ActionLink::goto(building_stage2));
}

struct SanWorkerFrame {
  decl_ptr: DeclPtr,
  counter: AtomicU16
}

fn sanitise_obj(handle : TaskHandle) -> ActionLink {
  let worker_frame =
    handle.interpret_frame::<SanWorkerFrame>();

  let obj_ptr = worker_frame.decl_ptr;
  let obj_kind = obj_ptr.project_tag();
  unsafe { match obj_kind {
    DeclKind::Definition => {
      let defn =
        &mut *obj_ptr.project_ptr().cast::<Definition>();
      let type_ = addr_of_mut!(defn.type_);
      let work =
      ActionLink::make_task_local_closure(&handle, type_, |type_, handle|{
        let type_ = *type_ ;
        let frame =
          handle.interpret_frame::<ExprWalkerFrame>();
        frame.node_ptr = type_;
        return ActionLink::goto(sanitise_expr)
      });
      let sub_framed =
        ActionLink::make_frame_request(DataFrameSize::Bytes56, work);
      handle.assign_work_for_schedule(sub_framed);
    },
    DeclKind::Mapping => {
      todo!()
    },
  } }

  return ActionLink::make_progress_checker(san_worker_awaiter);
}

fn san_worker_awaiter(handle : TaskHandle) -> Option<ActionLink> {
  let parrent_frame_handle =
    handle.get_parrent_frame().unwrap();
  let parrent_frame =
    parrent_frame_handle.interpret_frame::<MainFrame>();
  let worker_frame =
    handle.interpret_frame::<SanWorkerFrame>();

  if worker_frame.counter.load(Ordering::Relaxed) != 0 {
    return None
  }

  parrent_frame.counter.fetch_sub(1, Ordering::Relaxed);
  return Some(ActionLink::make_completion(true));
}

struct ExprWalkerFrame {
  node_ptr: *mut ExprPtr
}

// parent of this is san awaiter
fn sanitise_expr(handle : TaskHandle) -> ActionLink {
  let frame =
    handle.interpret_frame::<ExprWalkerFrame>();

  let node_ptr_ptr = frame.node_ptr ;
  let node_ptr = unsafe { *node_ptr_ptr };
  match node_ptr.project_presan_tag() {
    RawKind::Ref => todo!(),
    RawKind::App_ArgsInSlab => todo!(),
    RawKind::App_ArgsInline => todo!(),
    RawKind::App_ArgsInVec => todo!(),
    RawKind::Lam => todo!(),
    RawKind::Wit => todo!(),
    RawKind::Fun => todo!(),
    RawKind::Sig => todo!(),
    RawKind::Star => todo!(),
  }

  todo!()
}




fn building_stage2(handle : TaskHandle) -> ActionLink {


  return ActionLink::make_completion(true);
}