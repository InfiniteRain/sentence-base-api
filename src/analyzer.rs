extern crate serde;

use mecab::Tagger;
use rocket::serde::Serialize;

#[derive(Serialize)]
pub struct Morpheme {
    pub morpheme: String,
    pub dictionary_form: String,
    pub reading: String,
    pub features: Vec<String>,
}

pub fn dictionary_form_to_reading(dictionary_form: &str, default: String) -> String {
    let node_option = Tagger::new("")
        .parse_to_node(dictionary_form)
        .iter_next()
        .find(|node| {
            node.stat as i32 != mecab::MECAB_BOS_NODE && node.stat as i32 != mecab::MECAB_EOS_NODE
        });

    let node = match node_option {
        None => return default,
        Some(node) => node,
    };

    let features = node
        .feature
        .split(',')
        .map(|feature| feature.to_string())
        .collect::<Vec<String>>();

    match features.get(features.len().wrapping_sub(2)) {
        None => default,
        Some(reading) => reading.clone(),
    }
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
                features.clone(),
                features.remove(features.len().wrapping_sub(3)),
                features.remove(features.len().wrapping_sub(2)),
            )
        })
        .map(|(morpheme, features, dictionary_form, reading)| Morpheme {
            dictionary_form: if dictionary_form == "*" {
                morpheme.clone()
            } else {
                dictionary_form.clone()
            },
            reading: if reading == "*" {
                morpheme.clone()
            } else {
                dictionary_form_to_reading(&dictionary_form, reading)
            },
            morpheme,
            features,
        })
        .collect()
}
