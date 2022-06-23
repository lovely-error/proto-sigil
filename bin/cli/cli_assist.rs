use std::{
  io::{self, Write, Read}, path::{PathBuf}, slice,
  fs::{read_dir, ReadDir, File}};

use proto_sigil::parser::parser::ParsingState;

use crate::parser::CLIParseState;



pub struct KnotState {
  out: io::Stdout,
  inp: io::Stdin,
  recent_line: String,
  command_parser: Option<CLIParseState>,
  watched_directory: Option<ReadDir>
}

const CTRL : &str = "\u{1b}";
const DN : &str = "\u{1b}[0m";
const DIM : &str = "\u{1b}[2m";
const LogoColor : &str = "\u{1b}[38;5;203m";
const LineColor : &str = "\u{1b}[38;5;238m";
const RED : &str = "\u{1b}[38;5;126m";
const PROMPT : &str = "\u{1b}[38;5;126m";


impl KnotState {
  pub fn init() -> Self {
    Self {
      inp: io::stdin(),
      out: io::stdout(),
      recent_line: String::new(),
      command_parser: None,
      watched_directory: None,
    }
  }
  pub fn write_lines<const N : usize>(
    &mut self, lines: &[&str ; N], separator: Option<&str>
  ) {
    for line in lines {
      let _ = self.out.write((*line).as_bytes());
      if let Some(separator) = separator {
        let _ = self.out.write(separator.as_bytes());
      }
    }
  }
  pub fn write_line(&mut self, line: &str) {
    let _ = self.out.write((*line).as_bytes());
  }
  pub fn read_line(&mut self) {
    self.recent_line.clear();
    let _ = self.inp.read_line(&mut self.recent_line).unwrap();
    if let Some(ref mut cp) = self.command_parser {
      cp.set_new_command(
        self.recent_line.len() as u32);
      return;
    }
    if let None = self.command_parser {
      self.command_parser = Some(
        CLIParseState::init(
          self.recent_line.as_ptr(),
          self.recent_line.len() as u32));
    }
  }
  fn present_header(&mut self) {
    let mut padding = String::new();
    padding.reserve_exact(25);
    for _ in 0 .. 25 { padding.push(' ')}
    let logo =
      [ "▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄" ,
        "██ ▄▄▄ █▄ ▄██ ▄▄ █▄ ▄██ ████" ,
        "██▄▄▄▀▀██ ███ █▀▀██ ███ ████" ,
        "██ ▀▀▀ █▀ ▀██ ▀▀▄█▀ ▀██ ▀▀██" ,
        "▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀" ];
    let _ = self.out.write(LogoColor.as_bytes());
    for i in 0 .. logo.len() {
      let _ = self.out.write(padding.as_bytes());
      let _ = self.out.write(
        (*logo.get(i).unwrap()).as_bytes());
      let _ = self.out.write("\n".as_bytes());
    }
    let _ = self.out.write(DN.as_bytes());
    let v =  "Welcome to Sigilᵦ 0.1";
    let v_padding = {
      let size = 28;
      let mut padding = String::new();
      padding.reserve_exact(size);
      for _ in 0 .. size { padding.push(' ') }
      padding
    };
    let _ = self.out.write(v_padding.as_bytes());
    let _ = self.out.write(DIM.as_bytes());
    let _ = self.out.write(v.as_bytes());
    let _ = self.out.write(DN.as_bytes());
    let _ = self.out.write("\n\n".as_bytes());

  }
  fn prefix_match(&mut self, pattern: &str, should_strip: bool) -> bool {
    if let Some(ref mut parser) = self.command_parser {
      return parser.prefix_match(pattern, should_strip)
    }
    panic!("No data was read prior to invocation of the  command")
  }
  fn skip_whitespaces(&mut self) {
    if let Some(ref mut parser) = self.command_parser {
      parser.skip_whitespaces()
    } else {
      panic!("No data was read prior to invocation of the command")
    }
  }
  fn read_path(&mut self) -> Option<PathBuf> {
    if let Some(ref mut parser) = self.command_parser {
      if !parser.prefix_match("\"", true) {
        return None;
      }
      let start = parser.byte_index;
      parser.skip_while(|par|{
        let char = par.get_current_char();
        return char != '\"';
      });
      let end = parser.byte_index;
      let slice = unsafe {
        slice::from_raw_parts(
          parser.bytes.add(start as usize),
          (end - start)as usize) };
      let path =
        PathBuf::from(String::from_utf8(slice.to_vec()).unwrap());
      return Some(path);
    } else {
      panic!("No data was read prior to invocation of the command")
    }
  }
  pub fn run(&mut self) {
    self.write_line("\n");
    self.present_header();
    loop {
      //self.write_lines(&[DIM, ">", DN, " ", "\n"], None);
      self.read_line();
      match () {
        _ if self.prefix_match(":help", false) => {
          self.show_help_info();
        }
        _ if self.prefix_match(":watch", true) => {
          self.skip_whitespaces();
          self.watch_dir();
        }
        _ if self.prefix_match(":check", false) => {
          self.check_files();
        }
        _ => {
          let unrecognised_command_err = [
            RED, "Unrecognised command", DN, "\n"
          ];
          self.write_lines(&unrecognised_command_err, None);
        }
      }
      self.write_line("\n");
    }
  }
}


impl KnotState {
  fn show_help_info(&mut self) {
    let msg = [
      ":watch\n",
      "   " , DIM, "Sets folder that is watched in current session\n", DN,
      ":check\n",
      "   ", DIM, "Examines validity of watched definitions\n", DN,
      ":eval\n",
      "   ", DIM, "Performs reduction of a specified definition", DN, "\n"
    ];
    self.write_lines(&msg, None);
  }
  fn watch_dir(&mut self) {
    let path = self.read_path();
    if let None = path {
      let err = [
        RED, "Invalid path", DN, "\n"
      ];
      self.write_lines(&err, None);
      return;
    }
    let path = path.unwrap();
    if !path.exists() {
      let err = [
        RED, "Path doesnt refer to an existing directory", DN, "\n"
      ];
      self.write_lines(&err, None);
      return;
    }
    if !path.is_dir() {
      let err = [
        RED, "Path doesnt refer to a directory", DN, "\n"
      ];
      self.write_lines(&err, None);
      return;
    }
    let dir = read_dir(path);
    if let Err(_) = dir {
      let err = [
        RED, "Cant open directory", DN, "\n"
      ];
      self.write_lines(&err, None);
      return;
    }
    let dir = dir.unwrap();
    self.watched_directory = Some(dir);
  }
  fn check_files(&mut self) {
    if let Some(ref mut dir) = self.watched_directory {
      let mut source_code_files = Vec::new();
      for item in dir {
        if let Ok(ref item) = item {
          let path = item.path();
          let ext = path.extension();
          if let Some(ext) = ext {
            if ext == "sg" { source_code_files.push(path) }
          }
        }
      }
      self.process_files(source_code_files);
    } else {
      let err = [
        RED, "No directory was set for check", DN, "\n"
      ];
      self.write_lines(&err, None);
    }
  }
  fn process_files(&mut self, files: Vec<PathBuf>) {
    let mut decls = Vec::new();
    let mut chars = String::new();
    for file_path in files.iter() {
      let mut data = File::open(file_path).unwrap();
      let _ = data.read_to_string(&mut chars);
      let mut parser = ParsingState::init(
        chars.as_bytes());
      let smth = parser.run_parsing();
      match smth {
        Ok(things) => {
          for item in things {
            decls.push(item);
          }
        },
        Err(err) => {
          let msg = format!("{:#?}", err);
          let _ = self.out.write(msg.as_bytes());
        }
      }
    }
    let msg = format!("{:#?}", decls);
    let _ = self.out.write(msg.as_bytes());
  }
}