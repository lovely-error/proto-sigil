
pub trait Stream<Item> {
  fn next(&mut self) -> Option<Item>;
}

#[macro_export]
macro_rules! foreach {
  ($bind:pat in $iter:expr => $body:expr) => {
    {
      use proto_sigil::support_structures::traversing::Stream;
      loop {
        if let Some($bind) = Stream::next($iter) {
          $body
        } else { break };
      }
    }
  };
}

