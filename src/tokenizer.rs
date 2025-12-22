use anyhow::{Result, anyhow};
use std::sync::OnceLock;
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

static TOK: OnceLock<CoreBPE> = OnceLock::new();

/// Count tokens in a string using the shared CoreBPE tokenizer
#[inline]
pub fn count(text: &str) -> usize {
    let tok = TOK.get_or_init(|| {
        let model = std::env::var("CG_TOKENIZER_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.into());
        bpe_for_model(&model)
    });
    tok.encode_with_special_tokens(text).len()
}

/// Initialize the tokenizer model (call before any token counting).
pub fn init(model: Option<&str>) -> Result<()> {
    if TOK.get().is_some() {
        return Ok(());
    }
    let model = model
        .map(normalize_model_name)
        .or_else(|| std::env::var("CG_TOKENIZER_MODEL").ok())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    TOK.set(bpe_for_model(&model))
        .map_err(|_| anyhow!("tokenizer already initialized"))?;
    Ok(())
}
