//! 乱数（PCG32）は CPU の難易度でも使うので dsc-ai に置き、ここからはそれを使う
pub use dsc_ai::rng::Pcg32;
