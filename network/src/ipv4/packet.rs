use std::mem::transmute;
use std::num::Int;
use std::io::net::ip::{IpAddr, Ipv4Addr};
use std::vec::Vec;

#[deriving(PartialEq, PartialOrd, Eq, Ord,
           Clone, Show)]
pub struct V { buf: Vec<u8> }

pub struct A { buf:    [u8] }

#[deriving(PartialEq, PartialOrd, Eq, Ord,
           Clone, Show)]
pub enum BadPacket {
  TooShort,

  BadVersion,
  BadLength,
  BadChecksum,
  BadOptions,
}

fn make_header_checksum(header: &[u16]) -> u16 {
  let sum = header[0..5].iter().fold(0, |a, b| a + *b) +
    header[6..] .iter().fold(0, |a, b| a + *b);
  !(sum + (sum >> 12))
}

pub fn validate(packet: &[u8]) -> Result<(), BadPacket> {
  if packet.len() < MIN_HDR_LEN_BYTES as uint { return Err(TooShort) }
  Ok(())
}

impl V {
  pub fn new(buf: Vec<u8>) -> V {
    V { buf: buf }
  }

  /// NOT CHECKSUMED!
  fn new_with_header(ip:                 IpAddr,
                     protocol:           u8,
                     expected_body_size: Option<u16>) -> V
  {
    let mut buf: Vec<u8> = Vec::with_capacity(MIN_HDR_LEN_BYTES as uint
                                              + expected_body_size.unwrap_or(0) as uint);
    unsafe { buf.set_len(MIN_HDR_LEN_BYTES as uint); }
    let mut packet = V::new(buf);
    {
      let s = packet.borrow_mut();
      const SENTINAL8:  u8  = 0b_1100_0011;
      const SENTINAL16: u16 = 0b_1110_0000_0000_0111;
      const SENTINAL32: u32 = 0b_1111_0000_0000_0000_0000_0000_0000_1111;
      *(s.cast_h_mut()) = IpHeaderStruct {
        version_ihl:           SENTINAL8,  // SET LATER
        ///////////////////////////////////// Internet header length
        type_of_service:       SENTINAL8,  // SET LATER
        total_length:          SENTINAL16, // DO NOT SET
        identification:        SENTINAL16,
        flags_fragment_offset: SENTINAL16, // SET LATER
        ///////////////////////////////////// Fragment Offset
        time_to_live:          128,
        protocol:              protocol,
        header_checksum:       SENTINAL16, // DO NOT SET
        source_address:        SENTINAL32, // DO NOT SET
        destination_address:   SENTINAL32, // SET LATER
      };
      s.set_version(4);
      s.set_header_length(MIN_HDR_LEN_WORDS);
      s.set_type_of_service(Routine, ServiceFlags::empty());
      s.set_flags_fragment_offset(DONT_FRAGMENT, 0);
      {
        let u16s: &[u16] = unsafe { transmute(s.as_slice()) };
        // slice to make sure bod is cut,
        // and also because length is incorrect from transmute
        s.set_header_checksum(make_header_checksum(u16s[..12]));
      }
      let _ = s.set_destination(ip);
    }
    packet
  }

  // should be higher ranked liftime on closure, but that causes compiler ICE
  pub fn new_with_client
    <'clos, Err>
    (ip:                 IpAddr,
     protocol:           u8,
     expected_body_size: Option<u16>,
     client:             |&mut V|:'clos -> Result<(), Err>) -> Result<V, Err>
  {
    let mut packet = V::new_with_header(ip, protocol, expected_body_size);

    try!(client(&mut packet));

    let len = packet.borrow().as_slice().len() as u16;

    // once the new error handling libs land
    // this can be return Err(...) instead
    assert!(len > MIN_HDR_LEN_BYTES);

    // now fix header and checksum
    {
      let s = packet.borrow_mut();
      s.cast_h_mut().total_length = len;
      let u16s: &[u16] = unsafe { transmute(s.as_slice()) };
      // slice to make sure bod is cut,
      // and also because length is incorrect from transmute
      s.set_header_checksum(make_header_checksum(u16s[..12]));
    }
    Ok(packet)
  }

  pub fn as_vec(&self) -> &Vec<u8> { &self.buf }

  pub fn as_mut_vec(&mut self) -> &mut Vec<u8> { &mut self.buf }

  pub fn to_vec(self) -> Vec<u8> { self.buf }

  pub fn borrow(&self) -> &A { unsafe { transmute(self.buf.as_slice()) } }

  pub fn borrow_mut(&mut self) -> &mut A { unsafe { transmute(self.buf.as_mut_slice()) } }
}

pub const MIN_HDR_LEN_BITS:  u32 = MIN_HDR_LEN_WORDS as u32 * 32;
pub const MIN_HDR_LEN_BYTES: u16 = MIN_HDR_LEN_WORDS as u16 * 4;
pub const MIN_HDR_LEN_WORDS: u8  = 5;

///   From RFC 791
///
///    0                   1                   2                   3
///    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |Version|  IHL  |Type of Service|          Total Length         |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |         Identification        |Flags|      Fragment Offset    |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |  Time to Live |    Protocol   |         Header Checksum       |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |                       Source Address                          |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |                    Destination Address                        |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///   |                    Options                    |    Padding    |
///   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[repr(packed)]
#[unstable]
pub struct IpHeaderStruct {
  pub version_ihl:           u8,   // IP version (= 4)
  /////////////////////////////////// Internet header length
  pub type_of_service:       u8,   // Type of service
  pub total_length:          u16,  // Total length in octets
  pub identification:        u16,  // Identification
  pub flags_fragment_offset: u16,  // 3-bits Flags
  /////////////////////////////////// Fragment Offset
  pub time_to_live:          u8,   // Time To Live
  pub protocol:              u8,   // Protocol
  pub header_checksum:       u16,  // Checksum
  pub source_address:        u32,  // Source Address
  pub destination_address:   u32,  // Destination Address
}

pub struct IpStruct {
  pub header:           IpHeaderStruct,
  // mainly used to make struct a DST, so pointer can be cast
  pub options_and_body: [u8], // body of packet
}

#[repr(u8)]
pub enum Precedence {
  NetworkControl      = 0b111_00000,
  InternetworkControl = 0b110_00000,
  CriticEpc           = 0b101_00000,
  FlashOverride       = 0b100_00000,
  Flash               = 0b011_00000,
  Immediate           = 0b010_00000,
  Priority            = 0b001_00000,
  Routine             = 0b000_00000,
}

bitflags! {
  flags ServiceFlags: u8 {
    const LOW_DELAY         = 0b000_100_00,
    const HIGH_THROUGHPUT   = 0b000_010_00,
    const HIGH_RELIABILTY   = 0b000_001_00,
    //const NORMAL_DELAY      = !LowDelay       .bits,
    //const NORMAL_THROUGHPUT = !HighThroughput .bits,
    //const NORMAL_RELIABILTY = !HighReliability.bits,
  }
}

bitflags! {
  flags IpFlags: u16 {
    const DONT_FRAGMENT  = 0b010_00000_00000000,
    const MORE_FRAGMENTS = 0b001_00000_00000000,
  }
}


impl A {

  pub fn as_slice(&self) -> &[u8] {
    unsafe { transmute(self) }
  }

  pub fn as_mut_slice(&mut self) -> &mut [u8] {
    unsafe { transmute(self) }
  }

  pub fn cast(&self) -> &IpStruct {
    unsafe { transmute(self) }
  }

  pub fn cast_mut(&mut self) -> &mut IpStruct {
    unsafe { transmute(self) }
  }

  pub fn cast_h(&self) -> &IpHeaderStruct {
    &self.cast().header
  }

  pub fn cast_h_mut(&mut self) -> &mut IpHeaderStruct {
    &mut self.cast_mut().header
  }


  pub fn new(buf: &[u8]) -> &A {
    unsafe { transmute(buf) }
  }

  pub fn new_mut(buf: &mut [u8]) -> &mut A {
    unsafe { transmute(buf) }
  }

  pub fn get_version(&self) -> u8 { self.buf[0] >> 4 }
  pub fn set_version(&mut self, v: u8) {
    const MASK: u8 = 0b1111_0000;
    assert!(v & MASK == 0);
    self.buf[0] &= MASK;
    self.buf[0] |= v << 4;
  }

  pub fn get_header_length(&self) -> u8 { self.buf[0] & 0x0F }
  pub fn set_header_length(&mut self, v: u8) {
    const MASK: u8 = 0b1111_0000;
    assert!(v & MASK == 0);
    self.buf[0] |= v;
  }

  pub fn hdr_bytes(&self) -> u8 { self.get_header_length() * 4 }

  pub fn get_total_length(&    self) -> u16 { Int::from_be(self.cast_h()    .total_length) }
  pub fn set_total_length(&mut self, v: u16)             { self.cast_h_mut().total_length = v.to_be(); }


  pub fn get_type_of_service(&self) -> (Precedence, ServiceFlags) {
    const MASK: u8 = 0b111_00000;
    let tos = self.cast_h().type_of_service;
    ( unsafe { ::std::mem::transmute(tos & MASK) },
      ServiceFlags { bits: tos & !MASK } )
  }
  pub fn set_type_of_service(&mut self, prec: Precedence, flags: ServiceFlags) {
    self.cast_h_mut().type_of_service = prec as u8 | flags.bits;
  }


  pub fn get_identification(&    self) -> u16 { Int::from_be(self.cast_h()    .identification) }
  pub fn set_identification(&mut self, v: u16)             { self.cast_h_mut().identification = v.to_be(); }


  pub fn get_flags_fragment_offset(&self) -> (IpFlags, u16) {
    let ffo = self.cast_h().flags_fragment_offset;
    const MASK: u16 = 0b111_00000_00000000;
    ( unsafe { ::std::mem::transmute(ffo & MASK) },
      ffo & !MASK)
  }
  pub fn set_flags_fragment_offset(&mut self, flags: IpFlags, offset: u16) {
    assert!(0 == (offset & 0b111_00000_00000000));
    self.cast_h_mut().flags_fragment_offset = flags.bits | offset;
  }


  pub fn get_time_to_live(&    self) -> u8  { self.cast_h()    .time_to_live }
  pub fn set_time_to_live(&mut self, v: u8) { self.cast_h_mut().time_to_live = v; }

  pub fn get_protocol(&    self) -> u8  { self.cast_h()    .protocol }
  pub fn set_protocol(&mut self, v: u8) { self.cast_h_mut().protocol = v; }

  pub fn get_header_checksum(&    self) -> u16 { Int::from_be(self.cast_h()    .header_checksum) }
  pub fn set_header_checksum(&mut self, v: u16)             { self.cast_h_mut().header_checksum = v.to_be(); }

  pub fn get_source(&self) -> IpAddr {
    Ipv4Addr(self.buf[12], self.buf[13], self.buf[14], self.buf[15])
  }
  pub fn set_source(&mut self, ip: IpAddr) -> Result<(), ()> {
    match ip {
      Ipv4Addr(a, b, c, d) => {
        self.buf[12] = a;
        self.buf[13] = b;
        self.buf[14] = c;
        self.buf[15] = d;
      },
      _ => return Err(()),
    }
    Ok(())
  }

  pub fn get_destination(&self) -> IpAddr {
    Ipv4Addr(self.buf[16], self.buf[17], self.buf[18], self.buf[19])
  }
  pub fn set_destination(&mut self, ip: IpAddr) -> Result<(), ()> {
    match ip {
      Ipv4Addr(a, b, c, d) => {
        self.buf[16] = a;
        self.buf[17] = b;
        self.buf[18] = c;
        self.buf[19] = d;
      },
      _ => return Err(()),
    }
    Ok(())
  }

  // Eh, todo. Iterator over IpOptions?
  //pub fn options(&self) -> ... {  }

  pub fn get_payload(&self) -> &[u8] {
    if self.get_total_length() as uint > self.buf.len() {
      self.buf.slice_from(self.hdr_bytes() as uint)
    } else {
      self.buf.slice(self.hdr_bytes() as uint, self.get_total_length() as uint)
    }
  }

  //TODO: this results in when actually run
  //    task '<unknown>' has overflowed its stack
  //    Illegal instruction (core dumped)
  pub fn print(&self) {
    println!("Ip  | ver {} | {} | Tos {} | Len {}  |",
             self.get_version(), self.get_header_length(), self.cast_h().type_of_service, self.get_total_length());
    println!("    | FId {}    |   off {} |", self.get_identification(), self.get_flags_fragment_offset().val1());
    println!("    | ttl {} | proto {} | sum {} |", self.get_time_to_live(), self.get_protocol(), self.get_header_checksum());
    println!("    | Src {}   | Dst {} |", self.get_source(), self.get_destination());
  }
}
