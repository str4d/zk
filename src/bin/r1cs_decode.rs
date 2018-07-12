#[macro_use]
extern crate gumdrop;
extern crate zk;

use gumdrop::Options;
use std::fs::File;
use std::io::Read;

use zk::{
    r1cs::{Assignments, R1CS}, ConstraintSystem,
};

#[derive(Debug, Default, Options)]
struct MyOptions {
    #[options(help = "print help message")]
    help: bool,

    #[options(help = "Path to constraint system", meta = "FILE.r1cs")]
    r1cs: String,

    #[options(help = "Path to assignments", meta = "FILE.assignments")]
    assignments: String,
}

fn main() {
    let opts = MyOptions::parse_args_default_or_exit();
    let mut buf = Vec::new();

    if opts.r1cs.len() > 0 {
        match File::open(&opts.r1cs) {
            Ok(mut r1cs) => {
                buf.clear();
                r1cs.read_to_end(&mut buf).unwrap();
                let cs = R1CS::decode(&buf).unwrap();

                println!("> {}", &opts.r1cs);
                println!("{}", cs);
            }
            Err(e) => println!("Could not load {}: {}", &opts.r1cs, e),
        }
    }

    if opts.assignments.len() > 0 {
        match File::open(&opts.assignments) {
            Ok(mut assignments) => {
                buf.clear();
                assignments.read_to_end(&mut buf).unwrap();
                let a = Assignments::decode(&buf).unwrap();

                println!("> {}", &opts.assignments);
                println!("{}", a);
            }
            Err(e) => println!("Could not load {}: {}", &opts.assignments, e),
        }
    }
}
