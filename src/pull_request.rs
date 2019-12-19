//! Pull requests to `RustSec/advisory-db` on GitHub

use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::process::exit;

/// Parts of the pull request API response we're interested in parsing
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequest {
    /// Pull request's global primary key
    pub id: u64,

    /// URL where the pull request is located
    pub url: String,

    /// Title of the pull request
    pub title: String,

    /// Author of the pull request
    pub user: PullRequestUser,

    /// HEAD of the pull request
    pub head: PullRequestHead,

    /// Base commit for the given PR
    pub base: PullRequestBase,
}

impl PullRequest {
    /// Fetch a PR's info from api.github.com
    pub fn fetch(pull_request_id: u64) -> Self {
        let url = format!(
            "https://api.github.com/repos/rustsec/advisory-db/pulls/{}",
            pull_request_id
        );

        status_ok!("Fetching", "{}", url);

        let mut response = reqwest::get(&url).unwrap_or_else(|e| {
            status_err!(
                "couldn't get info about pull request #{}: {}",
                pull_request_id,
                e
            );
            exit(1);
        });

        if !response.status().is_success() {
            status_err!("api.github.com returned error: {}", response.status());
            exit(1);
        }

        serde_json::from_str(&response.text().unwrap()).unwrap_or_else(|e| {
            status_err!("error parsing response from https://api.github.com: {}", e);
            exit(1);
        })
    }

    /// Get the clone URL for this pull request
    pub fn clone_url(&self) -> &str {
        &self.head.repo.clone_url
    }
}

/// User who opened a pull request
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestUser {
    /// User's GitHub login
    pub login: String,
}

/// HEAD for the pull request
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestHead {
    /// Git ref (i.e. branch for the pull request)
    #[serde(rename = "ref")]
    pub git_ref: String,

    /// Repository where the pull request lives
    pub repo: PullRequestRepo,
}

/// Git repository a PR comes from
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestRepo {
    /// URL to clone the repository from
    pub clone_url: String,
}

/// Base commit for a pull request
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PullRequestBase {
    /// Commit hash the PR is based on
    pub sha: String,
}
