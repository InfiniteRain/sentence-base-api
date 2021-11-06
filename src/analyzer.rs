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
            let morpheme = (&(node.surface)[..(node.length as usize)]);
            let features = node.feature.split(',').collect::<Vec<&str>>();
            let dictionary_form = features
                .get(features.len().wrapping_sub(3))
                .unwrap_or(&morpheme);
            let reading = features
                .get(features.len().wrapping_sub(2))
                .unwrap_or(&morpheme);

            Morpheme {
                morpheme: morpheme.to_string(),
                dictionary_form: dictionary_form.to_string(),
                reading: reading.to_string(),
            }
        })
        .collect()
}
