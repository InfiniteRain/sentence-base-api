use serde_json::Value;
use std::collections::HashMap;

const JP_FREQUENCY_LIST: &str = include_str!("../frequency_lists/jp.json");

pub struct JpFrequencyList {
    lowest_frequency: usize,
    frequency_hash_map: HashMap<(String, String), usize>,
}

impl JpFrequencyList {
    pub fn new() -> Self {
        let mut frequency_hash_map: HashMap<(String, String), usize> = HashMap::new();
        let frequency_list_json: Value = serde_json::from_str(JP_FREQUENCY_LIST).unwrap();
        let words = frequency_list_json.as_array().unwrap();

        for (index, word_value) in words.iter().enumerate() {
            let word = word_value.as_array().unwrap();
            let dictionary_form = word[0].as_str().unwrap();
            let reading = word[1].as_str().unwrap();

            frequency_hash_map.insert((dictionary_form.to_string(), reading.to_string()), index);
        }

        JpFrequencyList {
            lowest_frequency: frequency_hash_map.len() + 1,
            frequency_hash_map,
        }
    }

    pub fn get_frequency(&self, word: &str, reading: &str) -> usize {
        *self
            .frequency_hash_map
            .get(&(word.to_string(), reading.to_string()))
            .unwrap_or(&self.lowest_frequency)
    }
}

impl Default for JpFrequencyList {
    fn default() -> Self {
        Self::new()
    }
}
