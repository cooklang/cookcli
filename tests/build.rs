use assert_cmd::Command;

#[test]
fn build_command_help_works() {
    let mut cmd = Command::cargo_bin("cook").unwrap();
    cmd.args(["build", "--help"]).assert().success();
}
