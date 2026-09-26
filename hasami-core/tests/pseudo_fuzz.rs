//! A deterministic "fuzz" run on stable Rust: a tiny PRNG generates a few
//! hundred thousand command lines from an alphabet chosen to hit every branch
//! of the lexer (`-`, `=`, plain letters, multi-byte UTF-8, ill-formed bytes,
//! WTF-8 surrogates) and runs the shared invariant harness over each.
//!
//! The real fuzzer lives in `fuzz/` and needs nightly + `cargo fuzz`.

mod common {
    pub mod harness;
}

use common::harness::exercise;

/// xorshift64*: no dependencies, good enough to spray inputs.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

const PIECES: &[&[u8]] = &[
    b"-",
    b"--",
    b"=",
    b"a",
    b"b",
    b"n",
    b"",
    b" ",
    b"\xff",
    b"\xfe",
    "é".as_bytes(),
    "日".as_bytes(),
    "😀".as_bytes(),
    b"\xED\xA0\x80",
    b"\xE3\x81",
    b"\xC3",
    b"--=",
    b"-a=",
    b"--long",
    b"--long=",
];

fn gen_arg(rng: &mut Rng) -> Vec<u8> {
    let pieces = rng.below(5);
    let mut out = Vec::new();
    for _ in 0..pieces {
        out.extend_from_slice(PIECES[rng.below(PIECES.len() as u64) as usize]);
    }
    out
}

fn gen_args(rng: &mut Rng) -> Vec<Vec<u8>> {
    let n = rng.below(7);
    (0..n).map(|_| gen_arg(rng)).collect()
}

fn gen_script(rng: &mut Rng) -> Vec<u8> {
    let n = 1 + rng.below(8);
    (0..n).map(|_| rng.below(256) as u8).collect()
}

#[test]
fn random_command_lines_uphold_invariants() {
    let iterations: u64 = std::env::var("HASAMI_PSEUDO_FUZZ_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200_000);
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut tokens = 0usize;
    for _ in 0..iterations {
        let args = gen_args(&mut rng);
        let script = gen_script(&mut rng);
        tokens += exercise(&args, &script);
    }
    assert!(tokens > 0);
}

#[test]
fn hand_picked_nasty_inputs() {
    let cases: &[&[&[u8]]] = &[
        &[b"--"],
        &[b"--", b"--"],
        &[b"-"],
        &[b"--="],
        &[b"--=="],
        &[b"-="],
        &[b"-=="],
        &[b"-\xff"],
        &[b"-\xff="],
        &[b"--\xff"],
        &[b"--\xff=\xff"],
        &[b"-\xED\xA0\x80"],
        &[b"-a\xED\xA0\x80b"],
        &[b"--a=\xED\xA0\x80"],
        &[b"-\xE3\x81"],
        &[b"-\xC3"],
        &[b""],
        &[b"", b"", b""],
        &[b"-", b"-", b"--", b"-"],
    ];
    for case in cases {
        let args: Vec<Vec<u8>> = case.iter().map(|a| a.to_vec()).collect();
        for script in [&[0u8][..], &[1], &[2], &[3], &[4], &[5], &[1, 2, 3, 4, 5]] {
            exercise(&args, script);
        }
    }
}
