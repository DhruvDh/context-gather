use once_cell::sync::Lazy;
use tiktoken_rs::{CoreBPE, get_bpe_from_model, o200k_base};

const DEFAULT_MODEL: &str = "gpt-5.2";

fn normalize_model_name(model: &str) -> String {
    model
        .to_lowercase()
        .chars()
        .map(|c| match c {
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2212}' => '-',
            _ => c,
        })
        .collect()
}

fn bpe_for_model(model: &str) -> CoreBPE {
    let normalized = normalize_model_name(model);
    if let Ok(bpe) = get_bpe_from_model(&normalized) {
        return bpe;
    }
    if normalized.starts_with("gpt-5") {
        return o200k_base().expect("tokenizer init failed");
    }
    o200k_base().expect("tokenizer init failed")
}

static TOK: Lazy<CoreBPE> = Lazy::new(|| bpe_for_model(DEFAULT_MODEL));

/// Count tokens in a string using the shared CoreBPE tokenizer
#[inline]
pub fn count(text: &str) -> usize {
    TOK.encode_with_special_tokens(text).len()
}
