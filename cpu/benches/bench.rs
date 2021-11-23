#![feature(test)]
extern crate crustationcpu;
extern crate test;

use crustationcpu::gte::Gte;

use test::Bencher;
use std::mem::replace;

// On my machine this hits 13 million RTPT/s and 10 million NCDT/s
// My understanding is that the original GTE did 120k triangles/s
// This should be good enough.

#[bench]
fn rtpt(b: &mut Bencher) {
    let mut gte: Gte = Gte::new();
    for i in 0..62 {
        gte.write_reg(i, 0xdead_beef);
    }

    b.iter(|| {
        let n = test::black_box(1000);

        (0..n).fold(0, |_, _| { gte.execute(0x00000030); 1 });
    })
}

#[bench]
fn mvmva(b: &mut Bencher) {
    let mut gte: Gte = Gte::new();
    for i in 0..62 {
        gte.write_reg(i, 0xdead_beef);
    }

    b.iter(|| {
        let n = test::black_box(1000);

        (0..n).fold(0, |_, _| { gte.execute(0x00000012); 1 });
    })
}

#[bench]
fn ncdt(b: &mut Bencher) {
    let mut gte: Gte = Gte::new();
    for i in 0..62 {
        gte.write_reg(i, 0xdead_beef);
    }

    b.iter(|| {
        let n = test::black_box(1000);

        (0..n).fold(0, |_, _| { gte.execute(0x00000016); 1 });
    })
}