pub mod dummy;
pub mod ring;
pub mod send;
pub mod recv;

pub trait PacketBuf
{
  fn new(init_seq_num: u32) -> Self;

  fn add_vec  (&mut self, seq_num: u32, vec: Vec<u8>, start_off: uint);
  fn add_slice(&mut self, seq_num: u32, buf: &[u8]);

  fn get_next_seq(&self) -> u32;
}
/*
pub trait PacketBufIter<'a>: PacketBuf
{
  type View:    Iterator<u8>;
  type Consume: Iterator<u8>;

  fn iter        (&'a     self) -> <Self as PacketBufIter<'a>>::View;
  fn consume_iter(&'a mut self) -> <Self as PacketBufIter<'a>>::Consume;
}
*/
