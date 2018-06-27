use std::io;

use super::ConstraintSystem;

mod encoding;

enum VariableIndex {
    Constant,
    Instance(usize),
    Witness(usize),
}

impl From<i64> for VariableIndex {
    fn from(i: i64) -> Self {
        match i {
            0 => VariableIndex::Constant,
            i if i < 0 => VariableIndex::Instance(-i as usize - 1),
            i => VariableIndex::Witness(i as usize - 1),
        }
    }
}

struct Coefficient(i64);

struct LinearCombination(Vec<(VariableIndex, Coefficient)>);

struct Constraint {
    a: LinearCombination,
    b: LinearCombination,
    c: LinearCombination,
}

struct Header {
    v: usize,
    p: usize,
    m: usize,
    nx: usize,
    nw: usize,
    _ignored: Vec<i64>,
}

impl Header {
    fn from_file(v: usize, n: Vec<i64>) -> Result<Self, ()> {
        macro_rules! parse_usize {
            ($value:expr) => {
                if $value < 0 {
                    return Err(());
                } else {
                    $value as usize
                }
            };
        }

        Ok(Header {
            v,
            p: parse_usize!(n[0]),
            m: parse_usize!(n[1]),
            nx: parse_usize!(n[2]),
            nw: parse_usize!(n[3]),
            _ignored: n[4..].to_vec(),
        })
    }
}

pub struct R1CS(Header, Vec<Constraint>);

impl ConstraintSystem for R1CS {
    fn decode(buf: &[u8]) -> io::Result<Self> {
        match encoding::r1cs(&buf[..]) {
            Ok((_, res)) => Ok(res),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to read R1CS file: {:?}", e),
            )),
        }
    }

    fn encode(&self) -> Vec<u8> {
        // TODO
        Vec::new()
    }
}
