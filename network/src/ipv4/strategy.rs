use std::option::Option;
use std::sync::Arc;


pub trait RoutingTable<'a>: Send + Sync + Sized {

  // initialized with the neighbor IPs
  fn init<I>(i: I) -> Self where I: Iterator<Item=super::Addr>;

  fn lookup(&self, super::Addr) -> Option<super::Addr>;

  fn monitor<E>(state: Arc<super::State<'a, Self, E>>) -> ();

  fn dump(&self);

}
