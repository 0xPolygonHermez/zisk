//! llama2.c's byte-pair-encoding tokenizer. Only the host uses it: the guest
//! works on token ids.
//!
//! `tokenizer.bin` layout: `i32 max_token_length`, then for each token
//! `f32 score`, `i32 len`, `u8 bytes[len]`.

use alloc::vec::Vec;

use crate::{BOS, read_f32, read_u32};

pub struct Tokenizer {
    vocab: Vec<Vec<u8>>,
    scores: Vec<f32>,
    /// Token ids sorted by their bytes, for lookups.
    sorted: Vec<u32>,
}

impl Tokenizer {
    pub fn parse(bytes: &[u8], vocab_size: usize) -> Result<Self, &'static str> {
        let mut pos = 4;
        let mut vocab = Vec::with_capacity(vocab_size);
        let mut scores = Vec::with_capacity(vocab_size);
        for _ in 0..vocab_size {
            if pos + 8 > bytes.len() {
                return Err("tokenizer file is truncated or does not match the model");
            }
            scores.push(read_f32(bytes, pos));
            let len = read_u32(bytes, pos + 4) as usize;
            pos += 8;
            if pos + len > bytes.len() {
                return Err("tokenizer file is truncated or does not match the model");
            }
            vocab.push(bytes[pos..pos + len].to_vec());
            pos += len;
        }
        let mut sorted: Vec<u32> = (0..vocab_size as u32).collect();
        sorted.sort_by(|&a, &b| vocab[a as usize].cmp(&vocab[b as usize]));
        Ok(Tokenizer { vocab, scores, sorted })
    }

    fn lookup(&self, piece: &[u8]) -> Option<u32> {
        self.sorted
            .binary_search_by(|&id| self.vocab[id as usize].as_slice().cmp(piece))
            .ok()
            .map(|i| self.sorted[i])
    }

    /// Encodes `text` as BOS followed by its tokens, like llama2.c's `encode`
    /// with `bos = 1, eos = 0`.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut tokens = Vec::from([BOS]);
        if text.is_empty() {
            return tokens;
        }
        // SentencePiece adds a leading space ("dummy prefix").
        if let Some(space) = self.lookup(b" ") {
            tokens.push(space);
        }
        // One token per character, or one per byte when the character is
        // not in the vocabulary (byte fallback: <0x00> is token 3).
        let mut buf = [0u8; 4];
        for ch in text.chars() {
            let piece = ch.encode_utf8(&mut buf).as_bytes();
            match self.lookup(piece) {
                Some(id) => tokens.push(id),
                None => tokens.extend(piece.iter().map(|&b| b as u32 + 3)),
            }
        }
        // Repeatedly merge the adjacent pair whose merge has the best score.
        let mut merged = Vec::new();
        loop {
            let mut best: Option<(f32, u32, usize)> = None;
            // llama2.c also considers the pair starting at BOS; it never merges.
            for i in 0..tokens.len() - 1 {
                merged.clear();
                merged.extend_from_slice(&self.vocab[tokens[i] as usize]);
                merged.extend_from_slice(&self.vocab[tokens[i + 1] as usize]);
                if let Some(id) = self.lookup(&merged) {
                    let score = self.scores[id as usize];
                    if best.is_none_or(|(s, _, _)| score > s) {
                        best = Some((score, id, i));
                    }
                }
            }
            let Some((_, id, i)) = best else { break };
            tokens[i] = id;
            tokens.remove(i + 1);
        }
        tokens
    }

    /// The bytes of `token` when it follows `prev`, like llama2.c's `decode`:
    /// the leading space after BOS is dropped, and `<0xXX>` pieces become
    /// that raw byte.
    pub fn decode(&self, prev: u32, token: u32) -> Vec<u8> {
        let mut piece: &[u8] = &self.vocab[token as usize];
        if prev == BOS && piece.first() == Some(&b' ') {
            piece = &piece[1..];
        }
        if piece.len() == 6 && piece.starts_with(b"<0x") && piece[5] == b'>' {
            let hex = core::str::from_utf8(&piece[3..5]).ok();
            if let Some(byte) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                return Vec::from([byte]);
            }
        }
        piece.to_vec()
    }
}
