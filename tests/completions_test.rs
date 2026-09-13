use assert_cmd::Command;
use predicates::prelude::*;

fn cook() -> Command {
    Command::cargo_bin("cook").unwrap()
}

#[test]
fn completions_bash_prints_a_bash_script() {
    cook()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"(?m)^\s*complete -F _cook .* cook$").unwrap());
}

#[test]
fn completions_zsh_prints_a_zsh_script() {
    cook()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("#compdef cook\n"));
}

#[test]
fn completions_fish_prints_a_fish_script() {
    cook()
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c cook "));
}

#[test]
fn completions_powershell_prints_a_powershell_script() {
    cook()
        .args(["completions", "powershell"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Register-ArgumentCompleter -Native -CommandName 'cook'",
        ));
}

#[test]
fn completions_elvish_prints_an_elvish_script() {
    cook()
        .args(["completions", "elvish"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "edit:completion:arg-completer[cook]",
        ));
}

#[test]
fn completions_script_covers_subcommands_and_their_flags() {
    // The script is generated from the live clap definition, so it must know
    // the subcommands, their aliases and their flags — not just the top level.
    cook()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shopping-list"))
        .stdout(predicate::str::contains(" shopping-list sl "))
        .stdout(predicate::str::contains("--base-path"));
}

#[test]
fn completions_rejects_an_unknown_shell() {
    cook()
        .args(["completions", "tcsh"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'tcsh'"));
}

#[test]
fn completions_requires_a_shell_argument() {
    cook()
        .arg("completions")
        .assert()
        .failure()
        .stderr(predicate::str::contains("<SHELL>"));
}

#[test]
fn completions_is_listed_in_top_level_help() {
    cook()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("completions"));
}
