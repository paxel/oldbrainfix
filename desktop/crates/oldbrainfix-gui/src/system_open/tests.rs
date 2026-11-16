use super::*;

#[test]
fn a_hostile_name_stays_one_argument_and_no_shell_reads_it() {
    let hostile = Path::new("/home/a/x\" & calc.exe & \"; rm -rf ~ $(id) `id` |sh.pdf");
    for macos in [false, true] {
        let (program, args) = argv(hostile, macos);
        assert!(!["cmd", "cmd.exe", "sh", "bash", "start"].contains(&program));
        assert_eq!(args, [hostile.as_os_str().to_owned()]);
    }
    assert_eq!(argv(hostile, true).0, "open");
    assert_eq!(argv(hostile, false).0, "xdg-open");
}

#[cfg(not(windows))]
#[test]
fn the_command_hands_the_path_over_as_it_is() {
    let path = Path::new("/tmp/a b;c.txt");
    let command = command(path).unwrap();
    let args: Vec<_> = command.get_args().collect();
    assert_eq!(args, [path.as_os_str()]);
}
