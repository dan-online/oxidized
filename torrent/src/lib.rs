use reqwest::Client;

// https://raw.githubusercontent.com/ngosang/trackerslist/master/trackers_best.txt
pub async fn get_best_trackers() -> Vec<String> {
    let url = "https://raw.githubusercontent.com/ngosang/trackerslist/master/trackers_best.txt";

    let client = Client::new();
    let response = client.get(url).send().await.unwrap();
    let body = response.text().await.unwrap();

    let trackers: Vec<String> = body
        .split("\n")
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();

    trackers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_best_trackers() {
        let trackers = get_best_trackers().await;
        assert_eq!(trackers.len(), 20);
    }
}
