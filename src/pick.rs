// pick.rs - Case-insensitive property picker

use std::collections::HashMap;

/// Picks specific properties from a HashMap with case-insensitive matching
pub fn pick(source: &HashMap<String, String>, properties: &[&str]) -> HashMap<String, String> {
    let mut result = HashMap::new();

    // Create a lowercase map of source keys
    let source_key_map: HashMap<String, &String> = source
        .iter()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect();

    for &prop in properties {
        if let Some(value) = source_key_map.get(&prop.to_lowercase()) {
            result.insert(prop.to_string(), (*value).clone());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pick_case_insensitive() {
        let mut source = HashMap::new();
        source.insert("User-Agent".to_string(), "Mozilla/5.0".to_string());
        source.insert("accept".to_string(), "image/webp".to_string());
        source.insert("REFERER".to_string(), "https://example.com".to_string());

        let result = pick(&source, &["user-agent", "Accept", "referer"]);

        assert_eq!(result.get("user-agent"), Some(&"Mozilla/5.0".to_string()));
        assert_eq!(result.get("Accept"), Some(&"image/webp".to_string()));
        assert_eq!(result.get("referer"), Some(&"https://example.com".to_string()));
    }

    #[test]
    fn test_pick_missing_keys() {
        let source = HashMap::new();
        let result = pick(&source, &["user-agent", "accept"]);
        assert!(result.is_empty());
    }
}
