use once_cell::sync::Lazy;
use tiktoken_rs::{CoreBPE, o200k_base};

static TOK: Lazy<CoreBPE> = Lazy::new(|| o200k_base().expect("tokenizer init failed"));

/// Count tokens in a string using the shared CoreBPE tokenizer
#[inline]
pub fn count(text: &str) -> usize {
    TOK.encode_with_special_tokens(text).len()
}
