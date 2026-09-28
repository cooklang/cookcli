//! `cook server user …` and `cook server hash-password`.

use super::users::{self, UsersDocument};
use anyhow::{bail, Context as _, Result};
use camino::Utf8PathBuf;
use clap::{Args, Subcommand};
use std::io::IsTerminal;

#[derive(Debug, Args)]
pub struct UserArgs {
    /// Users file to change
    ///
    /// Defaults to COOK_USERS_FILE when set, otherwise users.toml in the
    /// configuration directory -- the same file `cook server` reads.
    #[arg(long, value_name = "PATH", global = true, value_hint = clap::ValueHint::FilePath)]
    users_file: Option<Utf8PathBuf>,

    #[command(subcommand)]
    action: UserAction,
}

#[derive(Debug, Subcommand)]
enum UserAction {
    /// Add a user, prompting for their password
    ///
    /// Creates the users file if needed, which turns sign-in on the next time
    /// the server starts. A running server that already has a users file
    /// picks the new user up on its own.
    Add {
        /// Name to sign in with: letters, digits, and _ . @ -
        name: String,
    },
    /// Change a user's password, signing them out everywhere
    Passwd {
        /// User whose password to change
        name: String,
    },
    /// Remove a user, signing them out everywhere
    Remove {
        /// User to remove
        name: String,
    },
    /// List the users who can sign in
    List,
}

pub fn run_user(args: UserArgs) -> Result<()> {
    let location = users::locate(args.users_file.as_deref())?;
    let path = &location.path;
    let existing = match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(err).with_context(|| format!("could not read {path}")),
    };
    let mut doc = UsersDocument::parse(existing.as_deref().unwrap_or_default())
        .with_context(|| format!("invalid users file {path}"))?;

    match args.action {
        UserAction::List => {
            if existing.is_none() {
                eprintln!("No users file at {path}: sign-in is off.");
            }
            for name in doc.names() {
                println!("{name}");
            }
        }
        UserAction::Add { name } => {
            users::validate_username(&name)?;
            if doc.contains(&name) {
                bail!(
                    "{name} already exists in {path}; use `cook server user passwd {name}` \
                     to change their password"
                );
            }
            let hash = super::hash_password(&read_new_password()?)?;
            doc.set(&name, &hash);
            users::write_users_file(path, &doc.to_string())?;
            println!("Added {name} to {path}");
            if existing.is_none() {
                println!("Restart cook server to turn sign-in on.");
            }
        }
        UserAction::Passwd { name } => {
            if !doc.contains(&name) {
                bail!("there is no user {name} in {path}");
            }
            let hash = super::hash_password(&read_new_password()?)?;
            doc.set(&name, &hash);
            users::write_users_file(path, &doc.to_string())?;
            println!("Changed the password of {name} in {path}");
        }
        UserAction::Remove { name } => {
            if !doc.remove(&name) {
                bail!("there is no user {name} in {path}");
            }
            users::write_users_file(path, &doc.to_string())?;
            println!("Removed {name} from {path}");
            if doc.names().is_empty() {
                println!(
                    "No users left: nobody can make changes. Delete {path} and restart \
                     cook server to open it to everyone again."
                );
            }
        }
    }
    Ok(())
}

/// Prints the hash of a password, for editing the users file by hand.
pub fn run_hash_password() -> Result<()> {
    println!("{}", super::hash_password(&read_new_password()?)?);
    Ok(())
}

/// Asks for a new password twice without echoing it, or reads one line from
/// standard input when that is not a terminal, for scripts:
///
/// ```sh
/// printf '%s\n' "$PASSWORD" | cook server user add alice
/// ```
fn read_new_password() -> Result<String> {
    let password = if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
        // Prompts go to stderr: stdout may be redirected to capture a hash.
        let term = console::Term::stderr();
        term.write_str("Password: ")?;
        let first = term.read_secure_line()?;
        term.write_str("Repeat password: ")?;
        let second = term.read_secure_line()?;
        if first != second {
            bail!("the passwords do not match");
        }
        first
    } else {
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .context("could not read the password from standard input")?;
        line.trim_end_matches(['\r', '\n']).to_string()
    };

    if password.is_empty() {
        bail!("the password cannot be empty");
    }
    if password.len() > super::MAX_PASSWORD_LEN {
        bail!(
            "the password cannot be longer than {} bytes",
            super::MAX_PASSWORD_LEN
        );
    }
    Ok(password)
}
