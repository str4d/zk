#[macro_use]
extern crate nom;

use std::io;

pub mod r1cs;

pub trait ConstraintSystem: Sized {
    fn decode(&[u8]) -> io::Result<Self>;
    fn encode(&self) -> Vec<u8>;
}
