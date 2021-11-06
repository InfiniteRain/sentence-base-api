extern crate serde;

use mecab::Tagger;
use rocket::serde::Serialize;

#[derive(Serialize)]
pub struct Morpheme {
    pub morpheme: String,
    pub dictionary_form: String,
    pub reading: String,
}

pub fn analyze_sentence(sentence: &str) -> Vec<Morpheme> {
    Tagger::new("")
        .parse_to_node(sentence)
        .iter_next()
        .filter(|node| {
            node.stat as i32 != mecab::MECAB_BOS_NODE && node.stat as i32 != mecab::MECAB_EOS_NODE
        })
        .map(|node| {
            (
                (&(node.surface)[..(node.length as usize)]).to_string(),
                node.feature
                    .split(',')
                    .map(|feature| feature.to_string())
                    .collect::<Vec<String>>(),
            )
        })
        .map(|(morpheme, mut features)| {
            (
                morpheme,
                features.remove(features.len().wrapping_sub(3)),
                features.remove(features.len().wrapping_sub(2)),
            )
        })
        .map(|(morpheme, dictionary_form, reading)| Morpheme {
            dictionary_form: if dictionary_form == "*" {
                morpheme.clone()
            } else {
                dictionary_form
            },
            reading: if reading == "*" {
                morpheme.clone()
            } else {
                reading
            },
            morpheme,
        })
        .collect()
}
