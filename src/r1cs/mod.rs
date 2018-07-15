use std::fmt;
use std::io;

use super::ConstraintSystem;

mod encoding;

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
struct Coefficient(i64);

#[derive(Debug, PartialEq)]
struct LinearCombination(Vec<(VariableIndex, Coefficient)>);

impl LinearCombination {
    fn fmt(&self, f: &mut fmt::Formatter, char: usize) -> Result<(), fmt::Error> {
        if self.0.len() == 0 {
            write!(f, "0")
        } else {
            let char = char as i64;
            for (i, (v, c)) in self.0.iter().enumerate() {
                let (negate, k) = match c {
                    // To make the output cleaner, assume that field elements
                    // close to the characteristic are negative. This will
                    // mis-interpret truly-random coefficients as negative on
                    // occasion, but it's fine for display purposes.
                    Coefficient(k) if *k == char - 1 => (true, 1),
                    Coefficient(k) if *k == char - 2 => (true, 2),
                    Coefficient(k) if *k == char - 3 => (true, 3),
                    Coefficient(k) if *k == char - 4 => (true, 4),
                    Coefficient(k) if *k == char - 5 => (true, 5),
                    Coefficient(k) if *k == char - 6 => (true, 6),
                    Coefficient(k) if *k == char - 7 => (true, 7),
                    Coefficient(k) if *k == char - 8 => (true, 8),
                    Coefficient(k) if *k == char - 9 => (true, 9),
                    Coefficient(k) if *k == char - 10 => (true, 10),
                    // General cases
                    Coefficient(k) if *k < 0 => (true, -*k),
                    Coefficient(k) => (false, *k),
                };
                if negate {
                    if i > 0 {
                        write!(f, " - ")?
                    } else {
                        write!(f, "-")?
                    }
                } else if i > 0 {
                    write!(f, " + ")?
                }
                match v {
                    VariableIndex::Constant => write!(f, "{}", c.0)?,
                    VariableIndex::Instance(j) => match k {
                        1 => write!(f, "x_{}", j)?,
                        _ => write!(f, "x_{} * {}", j, k)?,
                    },
                    VariableIndex::Witness(j) => match k {
                        1 => write!(f, "w_{}", j)?,
                        _ => write!(f, "w_{} * {}", j, k)?,
                    },
                }
            }
            Ok(())
        }
    }
}

#[derive(Debug, PartialEq)]
struct Constraint {
    a: LinearCombination,
    b: LinearCombination,
    c: LinearCombination,
}

impl Constraint {
    fn fmt(&self, f: &mut fmt::Formatter, char: usize) -> Result<(), fmt::Error> {
        write!(f, "(")?;
        self.a.fmt(f, char)?;
        write!(f, ") * (")?;
        self.b.fmt(f, char)?;
        write!(f, ") = ")?;
        self.c.fmt(f, char)
    }
}

#[derive(Debug, PartialEq)]
struct Assignment(VariableIndex, i64);

impl fmt::Display for Assignment {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.0 {
            VariableIndex::Constant => write!(f, "Constant = {}", self.1)?,
            VariableIndex::Instance(i) => write!(f, "x_{} = {}", i, self.1)?,
            VariableIndex::Witness(i) => write!(f, "w_{} = {}", i, self.1)?,
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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

impl fmt::Display for R1CS {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Version:           {}\n", self.0.v)?;
        write!(f, "Characteristic:    {}\n", self.0.p)?;
        write!(f, "Degree:            {}\n", self.0.m)?;
        write!(f, "Input variables:   {}\n", self.0.nx)?;
        write!(f, "Witness variables: {}\n", self.0.nw)?;
        write!(f, "Constraints:\n")?;
        for c in &self.1 {
            write!(f, "  ")?;
            c.fmt(f, self.0.p)?;
            write!(f, "\n")?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct Assignments(Header, Vec<Assignment>);

impl Assignments {
    pub fn decode(buf: &[u8]) -> io::Result<Self> {
        match encoding::assignments(&buf[..]) {
            Ok((_, res)) => Ok(res),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to read assignments file",
            )),
        }
    }

    fn encode(&self) -> Vec<u8> {
        // TODO
        Vec::new()
    }
}

impl fmt::Display for Assignments {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Version:           {}\n", self.0.v)?;
        write!(f, "Characteristic:    {}\n", self.0.p)?;
        write!(f, "Degree:            {}\n", self.0.m)?;
        write!(f, "Input variables:   {}\n", self.0.nx)?;
        write!(f, "Witness variables: {}\n", self.0.nw)?;
        write!(f, "Assignments:\n")?;
        for a in &self.1 {
            write!(f, "  {}\n", a)?;
        }
        Ok(())
    }
}
