
mod cli_assist;


fn main () {
  println!("heyo");
  cli_assist::KnotState::init().run();
}