use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;

#[derive(Serialize, Deserialize)]
struct GteFuzzTest {
    name: String,
    input: Vec<String>,
    opcode: String,
    output: Vec<String>,
}

fn load_fuzz_tests(set_name: &str) -> Vec<GteFuzzTest> {
    let mut file = File::open("tests/gte/fuzz/data/".to_string() + set_name + ".json")
        .expect("Could not open fuzz data file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Could not load fuzz data");

    let tests: Vec<GteFuzzTest> =
        serde_json::from_str(&contents).expect("Could not parse fuzz data");
    tests
}

extern crate crustationcpu;
use crustationcpu::gte::{Flags, Gte};

#[test]
fn registers() {
    let tests = load_fuzz_tests("registers");

    for test in tests {
        let mut gte = Gte::new();

        for (r, val) in test.input.iter().enumerate() {
            let val = u32::from_str_radix(&val[2..], 16).unwrap();
            gte.write_reg(r as u32, val);
        }

        for (r, val) in test.output.iter().enumerate() {
            let expected = u32::from_str_radix(&val[2..], 16).unwrap();
            let actual = gte.read_reg(r as u32);

            assert_eq!(expected, actual);
        }
    }
}

fn run_gte_fuzz_test(test: &GteFuzzTest) -> bool {
    let mut gte = Gte::new();

    for (r, val) in test.input.iter().enumerate() {
        let val = u32::from_str_radix(&val[2..], 16).unwrap();
        gte.write_reg(r as u32, val);
    }

    let opcode = u32::from_str_radix(&test.opcode[2..], 16).unwrap();
    gte.execute(opcode);

    for (r, val) in test.output.iter().enumerate() {
        let expected = u32::from_str_radix(&val[2..], 16).unwrap();
        let actual = gte.read_reg(r as u32);

        if expected != actual {
            if r == 63 {
                let expected = Flags(expected);
                let actual = Flags(actual);

                println!("Error flags contains {:#?} but {:#?} was expected", actual, expected);
            } else {
                println!(
                    "r{} contains {:08x}, but was expecting {:08x}",
                    r, actual, expected
                );    
            }

            return false;
        }
    }

    true
}

fn run_gte_fuzz_suite(tests: Vec<GteFuzzTest>) {
    let n = std::env::args().last().unwrap();
    if let Ok(n) = usize::from_str_radix(&n, 10) {
        let test = &tests[n];
        println!("Running test {}: {}", n, test.name);
        assert_eq!(run_gte_fuzz_test(test), true);
    } else {
        let mut success = true;

        for (idx, test) in tests.iter().enumerate() {
            println!("Running test {}: {}", idx, test.name);
            if !run_gte_fuzz_test(test) {
                success = false;
            }
            println!()
        }

        assert_eq!(success, true);
    }
}

#[test]
fn avsz3() {
    let tests = load_fuzz_tests("avsz3");
    run_gte_fuzz_suite(tests);
}

#[test]
fn avsz4() {
    let tests = load_fuzz_tests("avsz4");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn cc() {
    let tests = load_fuzz_tests("cc");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn cdp() {
    let tests = load_fuzz_tests("cdp");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn dcpl() {
    let tests = load_fuzz_tests("dcpl");
    run_gte_fuzz_suite(tests);
}

#[test]
fn dpcs() {
    let tests = load_fuzz_tests("dpcs");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn dpct() {
    let tests = load_fuzz_tests("dpct");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn gpf() {
    let tests = load_fuzz_tests("gpf");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn gpl() {
    let tests = load_fuzz_tests("gpl");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn intpl() {
    let tests = load_fuzz_tests("intpl");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn mvmva() {
    let tests = load_fuzz_tests("mvmva");
    run_gte_fuzz_suite(tests);
}

#[test]
fn nccs() {
    let tests = load_fuzz_tests("nccs");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn ncct() {
    let tests = load_fuzz_tests("ncct");
    run_gte_fuzz_suite(tests);
}

#[test]
fn ncds() {
    let tests = load_fuzz_tests("ncds");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn ncdt() {
    let tests = load_fuzz_tests("ncdt");
    run_gte_fuzz_suite(tests);
}

#[test]
fn nclip() {
    let tests = load_fuzz_tests("nclip");
    run_gte_fuzz_suite(tests);
}

#[test]
fn ncs() {
    let tests = load_fuzz_tests("ncs");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn nct() {
    let tests = load_fuzz_tests("nct");
    run_gte_fuzz_suite(tests);
}

#[test]
fn op() {
    let tests = load_fuzz_tests("op");
    run_gte_fuzz_suite(tests);
}

#[test]
fn rptp() {
    let tests = load_fuzz_tests("rptp");
    run_gte_fuzz_suite(tests);
}

#[test]
fn rtps() {
    let tests = load_fuzz_tests("rtps");
    run_gte_fuzz_suite(tests);
}

#[test]
#[ignore]
fn sqr() {
    let tests = load_fuzz_tests("sqr");
    run_gte_fuzz_suite(tests);
}
