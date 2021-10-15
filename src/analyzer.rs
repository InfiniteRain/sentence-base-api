extern crate serde;

use mecab::Tagger;
use rocket::serde::Serialize;

#[derive(Serialize)]
pub struct Morpheme {
    pub morpheme: String,
    pub dictionary_form: String,
}

#[derive(Serialize)]
pub struct AnalysisResult {
    pub morphemes: Vec<Morpheme>,
}

pub fn analyze_sentence(sentence: &str) -> Vec<Morpheme> {
    Tagger::new("")
        .parse_to_node(sentence)
        .iter_next()
        .filter(|node| {
            node.stat as i32 != mecab::MECAB_BOS_NODE && node.stat as i32 != mecab::MECAB_EOS_NODE
        })
        .map(|node| Morpheme {
            morpheme: (&(node.surface)[..(node.length as usize)]).to_string(),
            dictionary_form: node.feature.split(',').collect::<Vec<&str>>()[6].to_string(),
        })
        .collect()
}
