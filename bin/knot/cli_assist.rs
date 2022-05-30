use std::io::{self, Write};


pub struct KnotState {
  out: io::Stdout,
  inp: io::Stdin,
}

const CTRL : char = '\u{1b}';
const DN : &str = "\u{1b}[0m";
const LogoColor : &str = "\u{1b}[38;5;203m";
const LineColor : &str = "\u{1b}[38;5;238m";


impl KnotState {
  pub fn init() -> Self {
    Self {
      inp: io::stdin(),
      out: io::stdout()
    }
  }
  fn present_header(&mut self) {
    let mut padding = String::new();
    padding.reserve_exact(25);
    for _ in 0 .. 25 { padding.push(' ')}
    let logo = [
      "▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄" ,
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
    let dim = "\u{1b}[2m";
    let v =  "Welcome to Sigilᵦ 0.1";
    let v_padding = {
      let size = 28;
      let mut padding = String::new();
      padding.reserve_exact(size);
      for _ in 0 .. size { padding.push(' ') }
      padding
    };
    let _ = self.out.write(v_padding.as_bytes());
    let _ = self.out.write(dim.as_bytes());
    let _ = self.out.write(v.as_bytes());
    let _ = self.out.write(DN.as_bytes());
    let _ = self.out.write("\n".as_bytes());

  }
  pub fn run(&mut self) {
    self.present_header();
    loop {
      let mut input = String::new();
      let _ = self.inp.read_line(&mut input).unwrap();
      if input == ":check\n" {
        let _ = self.out.write(input.as_bytes());
      }

    }
  }
}