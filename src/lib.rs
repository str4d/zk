#[macro_use]
extern crate cookie_factory;
#[macro_use]
extern crate nom;

use std::io;

pub mod r1cs;

pub trait ConstraintSystem: Sized {
    fn decode(&[u8]) -> io::Result<Self>;
    fn encode(&self) -> io::Result<Vec<u8>>;
}
