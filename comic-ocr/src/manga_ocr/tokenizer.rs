use std::path::Path;

use anyhow::{anyhow, Result};
use tokenizers::{
    models::wordpiece::{WordPiece, WordPieceBuilder},
    AddedToken, Tokenizer,
};

use super::load_json;
use crate::manga_ocr::load_json_from_bytes;
use std::collections::HashMap;

pub fn load_tokenizer_from_buf(
    vocab: &'static [u8],
    special_tokens: &'static [u8],
) -> Result<Tokenizer> {
    let mut mvocab = HashMap::new();
    let v = String::from_utf8_lossy(vocab);
    for (index, line) in v.lines().enumerate() {
        mvocab.insert(line.trim_end().to_owned(), index as u32);
    }
    let model = WordPieceBuilder::new()
        .vocab(mvocab)
        .build()
        .map_err(|e| anyhow!(e))?;

    let mut tokenizer = Tokenizer::new(model);
    let specials: serde_json::Value = load_json_from_bytes(special_tokens)?;
    let mut added = Vec::new();
    if let Some(obj) = specials.as_object() {
        for value in obj.values() {
            if let Some(token) = value.as_str() {
                added.push(AddedToken::from(token.to_string(), true));
            }
        }
    }
    if !added.is_empty() {
        tokenizer.add_special_tokens(&added);
    }

    Ok(tokenizer)
}

#[allow(dead_code)]
pub fn load_tokenizer(
    tokenizer_json: Option<&Path>,
    vocab_path: &Path,
    special_tokens_path: &Path,
) -> Result<Tokenizer> {
    if let Some(path) = tokenizer_json {
        if path.exists() {
            return Tokenizer::from_file(path).map_err(|e| anyhow!(e));
        }
    }

    let model = WordPiece::from_file(vocab_path.to_string_lossy().as_ref())
        .unk_token("[UNK]".to_string())
        .build()
        .map_err(|e| anyhow!(e))?;
    let mut tokenizer = Tokenizer::new(model);

    let specials: serde_json::Value = load_json(special_tokens_path)?;
    let mut added = Vec::new();
    if let Some(obj) = specials.as_object() {
        for value in obj.values() {
            if let Some(token) = value.as_str() {
                added.push(AddedToken::from(token.to_string(), true));
            }
        }
    }
    if !added.is_empty() {
        tokenizer.add_special_tokens(&added);
    }

    Ok(tokenizer)
}
