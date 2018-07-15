use cookie_factory::GenError;
use nom::IResult;

use super::{
    Assignment, Assignments, Coefficient, Constraint, Header, LinearCombination, R1CS,
    VariableIndex,
};

// VarInt
// - Each octet has MSB set to 1 if there is another octet, 0 otherwise.
// - The 7-bit groups are arranged in little-endian order.

fn usize_to_bits(mut n: usize) -> Vec<u8> {
    let mut res = Vec::new();
    while n > 127 {
        res.push((1 << 7) | (n & 127) as u8);
        n >>= 7;
    }
    res.push((n & 127) as u8);
    res
}

fn bits_to_usize(bits: (Vec<u8>, u8)) -> usize {
    let mut res = 0;
    let mut shift = 0;
    for i in 0..bits.0.len() {
        res += (bits.0[i] as usize) << shift;
        shift += 7;
    }
    res += (bits.1 as usize) << shift;
    res
}

named!(
    vlusize<usize>,
    bits!(do_parse!(
        res: many_till!(
            do_parse!(tag_bits!(u8, 1, 1) >> group: take_bits!(u8, 7) >> (group)),
            do_parse!(tag_bits!(u8, 1, 0) >> group: take_bits!(u8, 7) >> (group))
        ) >> (bits_to_usize(res))
    ))
);

fn gen_vlusize(input: (&mut [u8], usize), n: usize) -> Result<(&mut [u8], usize), GenError> {
    gen_slice!((input.0, input.1), usize_to_bits(n))
}

// SignedVarInt
// - Each octet has MSB set to 1 if there is another octet, 0 otherwise.
// - The 7-bit groups are arranged in little-endian order.
// - Integers are encoded by placing the sign bit in the LSB of the first
//   group, and all other bits shifted by 1 to the left.

fn i64_to_bits(n: i64) -> Vec<u8> {
    let mut n: u64 = ((n << 1) ^ (n >> 63)) as u64;
    let mut res = Vec::new();
    while n > 127 {
        res.push((1 << 7) | (n & 127) as u8);
        n >>= 7;
    }
    res.push((n & 127) as u8);
    res
}

fn bits_to_i64(bits: (Vec<u8>, u8)) -> i64 {
    let mut res = 0;
    let mut shift = 0;
    for i in 0..bits.0.len() {
        res += (bits.0[i] as u64) << shift;
        shift += 7;
    }
    res += (bits.1 as u64) << shift;
    if res & 1 == 0 {
        (res >> 1) as i64
    } else {
        -1 * ((res >> 1) + 1) as i64
    }
}

named!(
    vli64<i64>,
    bits!(do_parse!(
        res: many_till!(
            do_parse!(tag_bits!(u8, 1, 1) >> group: take_bits!(u8, 7) >> (group)),
            do_parse!(tag_bits!(u8, 1, 0) >> group: take_bits!(u8, 7) >> (group))
        ) >> (bits_to_i64(res))
    ))
);

fn gen_vli64(input: (&mut [u8], usize), n: i64) -> Result<(&mut [u8], usize), GenError> {
    gen_slice!((input.0, input.1), i64_to_bits(n))
}

// VariableIndex:
// SignedVarInt
// - Negative: instance variable
// - Zero: constant 1
// - Positive: witness variable

named!(
    variable_index<VariableIndex>,
    do_parse!(i: vli64 >> (VariableIndex::from(i)))
);

fn gen_variable_index<'a>(
    input: (&'a mut [u8], usize),
    i: &VariableIndex,
) -> Result<(&'a mut [u8], usize), GenError> {
    gen_vli64(input, i.into())
}

// Coefficient:
// Field element, represented as a SignedVarInt
// - Handles lots of small-value coefficients, and some random ones

named!(
    coefficient<Coefficient>,
    do_parse!(c: vli64 >> (Coefficient(c)))
);

fn gen_coefficient<'a>(
    input: (&'a mut [u8], usize),
    c: &Coefficient,
) -> Result<(&'a mut [u8], usize), GenError> {
    gen_vli64(input, c.0)
}

// Sequence:
// | Number of entries (VarInt) | Entry 0 | Entry 1 | … |

// LinearCombination
// | Sequence of (VariableIndex, Coefficient) |
// - Coefficients must be non-zero
// - Sorted by type, then index
//    - [constant, rev_sorted([instance]), sorted([witness])]

named!(
    linear_combination<LinearCombination>,
    do_parse!(
        pairs:
            length_count!(
                vlusize,
                do_parse!(i: variable_index >> c: coefficient >> ((i, c)))
            ) >> (LinearCombination(pairs))
    )
);

fn gen_linear_combination_entry<'a>(
    input: (&'a mut [u8], usize),
    entry: &(VariableIndex, Coefficient),
) -> Result<(&'a mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_variable_index(&entry.0) >> gen_coefficient(&entry.1)
    )
}

fn gen_linear_combination<'a>(
    input: (&'a mut [u8], usize),
    lc: &LinearCombination,
) -> Result<(&'a mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vlusize, lc.0.len()) >> gen_many_ref!(&lc.0, gen_linear_combination_entry)
    )
}

// R1CS constraint (A * B = C):
// | A: LinearCombination | B: LinearComination | C: LinearCombination |

named!(
    constraint<Constraint>,
    do_parse!(
        a: linear_combination
            >> b: linear_combination
            >> c: linear_combination
            >> (Constraint { a, b, c })
    )
);

fn gen_constraint<'a>(
    input: (&'a mut [u8], usize),
    c: &Constraint,
) -> Result<(&'a mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_linear_combination, &c.a)
            >> gen_call!(gen_linear_combination, &c.b)
            >> gen_call!(gen_linear_combination, &c.c)
    )
}

// Header:
// A version, followed by a Sequence of SignedVarInt.
// - Version (VarInt)
// - Number of SignedVarInts in the header (VarInt)
// - Field description
//   - Characteristic P
//   - Degree M
// - Number of instance variables N_X
// - Number of witness variables N_W
// - Further SignedVarInts are undefined in this spec, should be ignored
//
// | VERSION | HEADER_LENGTH | P | M | N_X | N_W |(... |)

named!(
    header<Header>,
    do_parse!(
        v: vlusize
            >> n: length_count!(vlusize, vli64)
            >> header: expr_res!(Header::from_file(v, n))
            >> (header)
    )
);

fn gen_header<'a>(
    input: (&'a mut [u8], usize),
    h: &Header,
) -> Result<(&'a mut [u8], usize), GenError> {
    let (v, n) = h.to_file();
    do_gen!(
        input,
        gen_call!(gen_vlusize, v) >> gen_call!(gen_vlusize, n.len()) >> gen_many!(n, gen_vli64)
    )
}

// R1CS file:
// | MAGICINT | Header | Sequence of R1CS constraints |

named!(
    pub r1cs<R1CS>,
    do_parse!(
        tag!("\x52\x31\x43\x53") >> h: header >> cs: length_count!(vlusize, constraint) >>
        (R1CS(h, cs))
    )
);

pub fn gen_r1cs<'a>(
    input: (&'a mut [u8], usize),
    r: &R1CS,
) -> Result<(&'a mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_slice!(&[0x52, 0x31, 0x43, 0x53])
            >> gen_call!(gen_header, &r.0)
            >> gen_call!(gen_vlusize, r.1.len())
            >> gen_many_ref!(&r.1, gen_constraint)
    )
}

// Assignments:
// An array of SignedVarInt, split up as follows:
// | 1 | x_0 | x_1 | … | x_(nx - 1) | w_0 | w_1 | … | w_(nw - 1) |
// nx and nw are defined by the header.

fn assignment_array(input: &[u8], nx: usize, nw: usize) -> IResult<&[u8], Vec<Assignment>> {
    do_parse!(
        input,
        c: count!(vli64, 1) >> x: count!(vli64, nx) >> w: count!(vli64, nw)
            >> (c.iter()
                .map(|n| Assignment(VariableIndex::Constant, *n))
                .chain(
                    x.iter()
                        .enumerate()
                        .map(|(j, n)| Assignment(VariableIndex::Instance(j), *n)),
                )
                .chain(
                    w.iter()
                        .enumerate()
                        .map(|(j, n)| Assignment(VariableIndex::Witness(j), *n)),
                )
                .collect())
    )
}

fn gen_assignment<'a>(
    input: (&'a mut [u8], usize),
    r: &Assignment,
) -> Result<(&'a mut [u8], usize), GenError> {
    gen_vli64(input, r.1)
}

// Assignments file:
// | MAGICINT | Header | Assignments |

named!(
    pub assignments<Assignments>,
    do_parse!(
        tag!("\x52\x31\x61\x73") >> h: header >> res: call!(assignment_array, h.nx, h.nw) >>
        (Assignments(h, res))
    )
);

pub fn gen_assignments<'a>(
    input: (&'a mut [u8], usize),
    r: &Assignments,
) -> Result<(&'a mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_slice!(&[0x52, 0x31, 0x61, 0x73])
            >> gen_call!(gen_header, &r.0)
            >> gen_many_ref!(&r.1, gen_assignment)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vlusize() {
        macro_rules! eval {
            ($value:expr, $expected:expr) => {
                let res = usize_to_bits($value);
                assert_eq!(&res, $expected);
                match vlusize(&res) {
                    Ok((_, n)) => assert_eq!(n, $value),
                    Err(e) => panic!("Unexpected error: {:?}", e),
                }
            };
        }

        eval!(0, &[0]);
        eval!(1, &[1]);
        eval!(2, &[2]);
        eval!(3, &[3]);
        eval!(127, &[127]);
        eval!(128, &[128, 1]);
        eval!(129, &[129, 1]);
        eval!(255, &[255, 1]);
        eval!(256, &[128, 2]);
        eval!(383, &[255, 2]);
        eval!(384, &[128, 3]);
        eval!(16383, &[255, 127]);
        eval!(16384, &[128, 128, 1]);
        eval!(16385, &[129, 128, 1]);
        eval!(65535, &[255, 255, 3]);
        eval!(65536, &[128, 128, 4]);
        eval!(65537, &[129, 128, 4]);
        eval!(2097151, &[255, 255, 127]);
        eval!(2097152, &[128, 128, 128, 1]);
        eval!(2097153, &[129, 128, 128, 1]);
    }

    #[test]
    fn test_vli64() {
        macro_rules! eval {
            ($value:expr, $expected:expr) => {
                let res = i64_to_bits($value);
                assert_eq!(&res, $expected);
                match vli64(&res) {
                    Ok((_, n)) => assert_eq!(n, $value),
                    Err(e) => panic!("Unexpected error: {:?}", e),
                }
            };
        }

        eval!(0, &[0]);
        eval!(-1, &[1]);
        eval!(1, &[2]);
        eval!(-2, &[3]);
        eval!(2, &[4]);
        eval!(-63, &[125]);
        eval!(63, &[126]);
        eval!(-64, &[127]);
        eval!(64, &[128, 1]);
        eval!(-65, &[129, 1]);
        eval!(-128, &[255, 1]);
        eval!(128, &[128, 2]);
        eval!(-192, &[255, 2]);
        eval!(192, &[128, 3]);
        eval!(-8192, &[255, 127]);
        eval!(8192, &[128, 128, 1]);
        eval!(-8193, &[129, 128, 1]);
        eval!(-32768, &[255, 255, 3]);
        eval!(32768, &[128, 128, 4]);
        eval!(-32769, &[129, 128, 4]);
        eval!(-1048576, &[255, 255, 127]);
        eval!(1048576, &[128, 128, 128, 1]);
        eval!(-1048577, &[129, 128, 128, 1]);
    }
}
