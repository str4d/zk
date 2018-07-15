use cookie_factory::GenError;
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

impl<'a> From<&'a VariableIndex> for i64 {
    fn from(i: &VariableIndex) -> Self {
        match i {
            &VariableIndex::Constant => 0,
            &VariableIndex::Instance(i) => -(i as i64 + 1),
            &VariableIndex::Witness(i) => i as i64 + 1,
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

    fn to_file(&self) -> (usize, Vec<i64>) {
        let mut n = Vec::with_capacity(4 + self._ignored.len());
        n.push(self.p as i64);
        n.push(self.m as i64);
        n.push(self.nx as i64);
        n.push(self.nw as i64);
        n.extend_from_slice(&self._ignored);
        (self.v, n)
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

    fn encode(&self) -> io::Result<Vec<u8>> {
        let mut data = Vec::new();
        loop {
            match encoding::gen_r1cs((&mut data, 0), self) {
                Ok(_) => return Ok(data),
                Err(e) => match e {
                    GenError::BufferTooSmall(sz) => {
                        data.resize(sz, 0);
                        continue;
                    }
                    GenError::InvalidOffset
                    | GenError::CustomError(_)
                    | GenError::NotYetImplemented => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "could not encode R1CS",
                        ))
                    }
                },
            }
        }
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

    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut data = Vec::new();
        loop {
            match encoding::gen_assignments((&mut data, 0), self) {
                Ok(_) => return Ok(data),
                Err(e) => match e {
                    GenError::BufferTooSmall(sz) => {
                        data.resize(sz, 0);
                        continue;
                    }
                    GenError::InvalidOffset
                    | GenError::CustomError(_)
                    | GenError::NotYetImplemented => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "could not encode Assignments",
                        ))
                    }
                },
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r1cs_encode_decode() {
        // Simple XOR circuit:
        //   Version:           0
        //   Characteristic:    64513
        //   Degree:            1
        //   Input variables:   1
        //   Witness variables: 2
        //   Constraints:
        //     (1 - w_0) * (w_0) = 0
        //     (1 - w_1) * (w_1) = 0
        //     (w_0 * 2) * (w_1) = -x_0 + w_0 + w_1
        let header = Header::from_file(0, vec![64513, 1, 1, 2]).unwrap();
        let constraints = vec![
            Constraint {
                a: LinearCombination(vec![
                    (VariableIndex::Constant, Coefficient(1)),
                    (VariableIndex::Witness(0), Coefficient(-1)),
                ]),
                b: LinearCombination(vec![(VariableIndex::Witness(0), Coefficient(1))]),
                c: LinearCombination(vec![(VariableIndex::Constant, Coefficient(0))]),
            },
            Constraint {
                a: LinearCombination(vec![
                    (VariableIndex::Constant, Coefficient(1)),
                    (VariableIndex::Witness(1), Coefficient(-1)),
                ]),
                b: LinearCombination(vec![(VariableIndex::Witness(1), Coefficient(1))]),
                c: LinearCombination(vec![(VariableIndex::Constant, Coefficient(0))]),
            },
            Constraint {
                a: LinearCombination(vec![(VariableIndex::Witness(0), Coefficient(2))]),
                b: LinearCombination(vec![(VariableIndex::Witness(1), Coefficient(1))]),
                c: LinearCombination(vec![
                    (VariableIndex::Instance(0), Coefficient(-1)),
                    (VariableIndex::Witness(0), Coefficient(1)),
                    (VariableIndex::Witness(1), Coefficient(1)),
                ]),
            },
        ];
        let r1cs = R1CS(header, constraints);

        let encoded = r1cs.encode().unwrap();
        let decoded = R1CS::decode(&encoded);
        assert_eq!(decoded.unwrap(), r1cs);
    }

    #[test]
    fn assignments_encode_decode() {
        // Assignments for the simple XOR circuit above:
        //   Version:           0
        //   Characteristic:    64513
        //   Degree:            1
        //   Input variables:   1
        //   Witness variables: 2
        //   Assignments:
        //     Constant = 1
        //     x_0 = 1
        //     w_0 = 0
        //     w_1 = 1
        let header = Header::from_file(0, vec![64513, 1, 1, 2]).unwrap();
        let assignments = vec![
            Assignment(VariableIndex::Constant, 1),
            Assignment(VariableIndex::Instance(0), 1),
            Assignment(VariableIndex::Witness(0), 0),
            Assignment(VariableIndex::Witness(1), 1),
        ];
        let assignments = Assignments(header, assignments);

        let encoded = assignments.encode().unwrap();
        let decoded = Assignments::decode(&encoded);
        assert_eq!(decoded.unwrap(), assignments);
    }
}
