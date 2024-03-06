use reqwest::Client;
use tokio::time::Instant;
use tracing::{info, warn};

pub struct NSFWFilter {
    words: Vec<String>,
    last_updated: Instant,
}

impl NSFWFilter {
    pub async fn new() -> Self {
        let mut new_self = Self {
            words: vec![],
            last_updated: Instant::now(),
        };

        new_self.get_words().await;

        new_self
    }

    pub async fn test(&mut self, text: &str) -> bool {
        let words = self.get_words().await;

        for word in words {
            if text.to_lowercase().contains(&word.to_lowercase()) {
                return true;
            }
        }

        false
    }

    async fn make_request(
        &self,
        client: Client,
        url: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let response = client.get(url).send().await?;

        Ok(response)
    }

    pub async fn fetch_words(&self) -> anyhow::Result<Vec<String>> {
        let mirrors = vec![
            "https://raw.githubusercontent.com/LDNOOBW/List-of-Dirty-Naughty-Obscene-and-Otherwise-Bad-Words/master/en",
            "https://cdn.jsdelivr.net/gh/LDNOOBW/List-of-Dirty-Naughty-Obscene-and-Otherwise-Bad-Words@master/en",
        ];

        let client = Client::new();
        let mut all = vec![];

        for url in mirrors {
            let response = self.make_request(client.clone(), url).await;

            if let Ok(response) = response {
                let body = response.text().await.unwrap();

                let words: Vec<String> = body
                    .split("\n")
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().to_string())
                    .collect();

                all.extend(words);

                info!("Fetched {} bad-words", all.len());

                return Ok(all);
            }

            warn!(
                "Cannot fetch bad-words list from {}, moving to next mirror: {:?}",
                url,
                response.err()
            );
        }

        Ok(all)
    }

    pub async fn get_words(&mut self) -> Vec<String> {
        if self.last_updated.elapsed().as_secs() > 86400 || self.words.is_empty() {
            match self.fetch_words().await {
                Ok(words) => {
                    self.words = words;
                    self.last_updated = Instant::now();
                }
                Err(e) => {
                    warn!("Cannot fetch bad-words list: {:?}", e);
                }
            }
        }

        return self.words.clone();
    }
}
