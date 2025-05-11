#![feature(test)]
extern crate crustationcpu;
extern crate test;

use crustationcpu::gte::Gte;

use test::Bencher;

#[bench]
fn rtpt(b: &mut Bencher) {
    let mut gte: Gte = Gte::new();
    for i in 0..62 {
        gte.write_reg(i, 0xdead_beef);
    }

    b.iter(|| {
        gte.execute(0x00000030);
    })
}

#[bench]
fn mvmva(b: &mut Bencher) {
    let mut gte: Gte = Gte::new();
    for i in 0..62 {
        gte.write_reg(i, 0xdead_beef);
    }

    b.iter(|| {
        gte.execute(0x00000012);
    })
}

#[bench]
fn ncdt(b: &mut Bencher) {
    let mut gte: Gte = Gte::new();
    for i in 0..62 {
        gte.write_reg(i, 0xdead_beef);
    }

    b.iter(|| {
        gte.execute(0x00000016);
    })
}
