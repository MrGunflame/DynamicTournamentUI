pub mod tournament;

use crate::tournament::Tournaments;

#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
}

impl Client {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub fn tournaments(&self) -> Tournaments<'_> {
        Tournaments::new(self)
    }
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;