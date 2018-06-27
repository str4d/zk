use std::io;

pub trait ConstraintSystem: Sized {
    fn decode(&[u8]) -> io::Result<Self>;
    fn encode(&self) -> Vec<u8>;
}
