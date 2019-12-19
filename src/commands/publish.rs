//! `rustsec-admin publish` subcommand
//!
//! Publishes an advisory from a PR

use crate::{prelude::*, pull_request::PullRequest, error::Error};
use abscissa_core::{Command, Runnable};
use gumdrop::Options;
use rustsec::repository::authentication::with_authentication;
use std::{
    path::{Path, PathBuf},
    process::exit,
};

/// git ref for the master branch of `RustSec/advisory-db`
const LOCAL_MASTER_REF: &str = "refs/heads/master";

/// `rustsec-admin publish` subcommand
#[derive(Command, Debug, Default, Options)]
pub struct PublishCmd {
    /// Filesystem path to the advisory database git repository
    #[options(
        long = "db",
        help = "advisory database git repo path (default: ~/.cargo/advisory-db)"
    )]
    db: Option<PathBuf>,

    /// Pull request ID number
    #[options(free, help = "pull request ID number")]
    pull_request_id: Vec<String>,
}

impl Runnable for PublishCmd {
    fn run(&self) {
        let repo_path = self
            .db
            .as_ref()
            .map(AsRef::as_ref)
            .unwrap_or(Path::new("."));

        let database = fetch_database_repo(rustsec::repository::DEFAULT_URL, repo_path);

        status_ok!(
            "Loaded",
            "{} security advisories (from {})",
            database.iter().len(),
            repo_path.display()
        );

        let git_repo = git2::Repository::open(repo_path).unwrap_or_else(|e| {
            status_err!("error opening advisory DB git repo: {}", e);
            exit(1);
        });

        if self.pull_request_id.len() != 1 {
            status_err!("publish requires one argument: GitHub pull request ID number to publish");
            exit(1);
        }

        let pull_request_id: u64 = self.pull_request_id[0].parse().unwrap_or_else(|e| {
            status_err!("error parsing pull request ID number: {}", e);
            exit(1);
        });

        let pull_request = PullRequest::fetch(pull_request_id);

        status_ok!(
            "Retrieved",
            "pull request #{} \"{}\" (by {})",
            pull_request_id,
            &pull_request.title,
            &pull_request.user.login
        );

        let remote_ref = configure_git_remote(&git_repo, &pull_request);
        dbg!(&remote_ref);

        pull_remote_repo(&git_repo, &pull_request, &remote_ref).unwrap_or_else(|e| {
            status_err!("couldn't pull {}: {}", pull_request.clone_url(), e);
            exit(1);
        })
    }
}

/// Fetch and load the local advisory DB repo
fn fetch_database_repo(url: &str, path: &Path) -> rustsec::Database {
    status_ok!("Pulling", "advisory database from `{}`", url);

    let repo = rustsec::Repository::fetch(url, path, true).unwrap_or_else(|e| {
        status_err!("couldn't fetch advisory database: {}", e);
        exit(1);
    });

    rustsec::Database::load(&repo).unwrap_or_else(|e| {
        status_err!(
            "error loading advisory database from {}: {}",
            path.display(),
            e
        );
        exit(1);
    })
}

/// Configure the git remote for the source repo in the PR
fn configure_git_remote(git_repo: &git2::Repository, pull_request: &PullRequest) -> String {
    let remote_name = &pull_request.user.login;

    if let Ok(remote) = git_repo.find_remote(remote_name) {
        if remote.url() != Some(pull_request.clone_url()) {
            status_err!(
                    "git remote '{}' exists but has different URL (expected {:?}, got {:?})",
                    &pull_request.user.login,
                    pull_request.clone_url(),
                    remote.url()
                );
            exit(1);
        }
    } else {
        git_repo
            .remote(remote_name, pull_request.clone_url())
            .unwrap_or_else(|e| {
                status_err!(
                    "error adding remote for '{}': {}",
                    &pull_request.user.login,
                    e
                );
                exit(1);
            });

        status_info!(
            "Added",
            "git remote '{}': {}",
            &pull_request.user.login,
            pull_request.clone_url()
        );
    }

    // Return the git remote path
    return format!("remotes/{}", remote_name);
}

/// Pull the new advisory from the third party remote
fn pull_remote_repo(git_repo: &git2::Repository, pull_request: &PullRequest, remote_ref: &str) -> Result<(), Error> {
    let git_config = git2::Config::new()?;
    let pull_request_ref = [remote_ref, &pull_request.head.git_ref].join("/");

    with_authentication(pull_request.clone_url(), &git_config, |f| {
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(f);

        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let refspec = LOCAL_MASTER_REF.to_owned() + ":" + &pull_request_ref;
        dbg!(&refspec);

        // Fetch remote packfiles and update tips
        let mut remote = git_repo.remote_anonymous(pull_request.clone_url())?;
        remote.fetch(&[refspec.as_str()], Some(&mut fetch_opts), None)?;

        // Get the current remote tip (as an updated local reference)
        let remote_master_ref = git_repo.find_reference(&pull_request_ref)?;
        let remote_target = remote_master_ref.target().unwrap();

        dbg!(&remote_target);

        // Set the local master ref to match the remote
        //let mut local_master_ref = git_repo.find_reference(LOCAL_MASTER_REF)?;
        //local_master_ref.set_target(
        //    remote_target,
        //    &format!(
        //        "rustsec: moving master to {}: {}",
        //        REMOTE_MASTER_REF, &remote_target
        //    ),
        //)?;

        Ok(())
    })?;

    Ok(())
}
