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
    let mut tagger = Tagger::new("");
    let mut morphemes: Vec<Morpheme> = vec![];

    for node in tagger.parse_to_node(sentence).iter_next() {
        let node_stat = node.stat as i32;
        if node_stat == mecab::MECAB_BOS_NODE || node_stat == mecab::MECAB_EOS_NODE {
            continue;
        }

        let features: Vec<&str> = node.feature.split(',').collect();
        let morpheme = &(node.surface)[..(node.length as usize)];
        let dictionary_form = features[6];

        morphemes.push(Morpheme {
            morpheme: morpheme.to_string(),
            dictionary_form: dictionary_form.to_string(),
        });
    }

    morphemes
}
