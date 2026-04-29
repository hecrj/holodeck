pub mod tcgdex;

use tcgdex::Tcgdex;

use std::sync::LazyLock;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct Session {
    pub tcgdex: Tcgdex,
}

impl Session {
    pub fn new() -> Self {
        Self {
            tcgdex: Tcgdex::new(),
        }
    }
}

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("Build reqwest client")
});

async fn retry<T, F>(mut retries: usize, f: impl Fn() -> F) -> Result<T, reqwest::Error>
where
    F: Future<Output = Result<T, reqwest::Error>>,
{
    loop {
        let result = f().await;

        match result {
            Ok(response) => {
                break Ok(response);
            }
            Err(error) if error.status() == Some(reqwest::StatusCode::NOT_FOUND) => {
                break Err(error);
            }
            Err(error) => {
                if retries > 0 {
                    log::warn!(
                        "{error} ({retries} {} left)",
                        if retries == 1 { "retry" } else { "retries" }
                    );
                    retries -= 1;
                } else {
                    break Err(error);
                }
            }
        }
    }
}
