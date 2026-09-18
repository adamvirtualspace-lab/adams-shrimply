use super::Time;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentColor {
    Red,
    Orange,
    #[default]
    Yellow,
    Green,
    Blue,
    Purple,
}

impl CommentColor {
    pub const ALL: [Self; 6] = [
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Purple,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommentItem {
    pub id: Uuid,
    pub start: Time,
    pub end: Time,
    pub text: String,
    #[serde(default)]
    pub color: CommentColor,
    #[serde(default)]
    pub group_id: Option<u64>,
}

impl CommentItem {
    pub fn new(start: Time, end: Time, text: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            start,
            end,
            text,
            color: CommentColor::default(),
            group_id: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommentTrack {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "super::default_true")]
    pub enabled: bool,
    pub items: Vec<CommentItem>,
}

impl Default for CommentTrack {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            enabled: true,
            items: Vec::new(),
        }
    }
}
