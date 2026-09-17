use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut unique = Vec::new();
    for tag in tags {
        let tag = tag.trim();
        if !tag.is_empty() && !unique.iter().any(|existing| existing == tag) {
            unique.push(tag.to_owned());
        }
    }
    unique
}

pub(super) fn deserialize_tags<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<String>, D::Error> {
    Vec::<String>::deserialize(deserializer).map(|tags| normalize_tags(&tags))
}

pub(super) fn serialize_tags<S: Serializer>(
    tags: &[String],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    normalize_tags(tags).serialize(serializer)
}
