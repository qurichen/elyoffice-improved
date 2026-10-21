use std::{
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Output, Stdio},
    time::Duration,
};

use ely_platform::{AppId, instance::Start, settings::Store, storage::Kind};

fn command(root: Option<&Path>, mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "profile_probe", "--ignored", "--nocapture"])
        .env("ELY_PROFILE_PROBE", mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match root {
        Some(path) => {
            command.env("ELY_DATA_DIR", path);
        }
        None => {
            command.env_remove("ELY_DATA_DIR");
        }
    }
    command
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

struct Holder {
    child: Child,
    root: PathBuf,
    out: BufReader<ChildStdout>,
}

impl Holder {
    fn start(root: &Path) -> Self {
        let mut child = command(Some(root), "hold")
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut holder = Self {
            child,
            root: root.to_owned(),
            out,
        };
        let ready = holder
            .out
            .by_ref()
            .lines()
            .map(Result::unwrap)
            .any(|line| line == "PROFILE_HELD");
        assert!(ready, "the isolated holder did not start");
        holder
    }

    fn finish(mut self) {
        std::fs::write(self.root.join("release"), b"").unwrap();
        let mut said = String::new();
        self.out.read_to_string(&mut said).unwrap();
        assert!(self.child.wait().unwrap().success(), "{said}");
    }
}

impl Drop for Holder {
    fn drop(&mut self) {
        if self.child.try_wait().unwrap().is_none() {
            self.child.kill().unwrap();
            self.child.wait().unwrap();
        }
    }
}

#[test]
fn absent_override_only_queries_the_default() {
    let said = success(command(None, "paths").output().unwrap());
    let expected = directories::BaseDirs::new()
        .unwrap()
        .data_local_dir()
        .join("ElyOffice");
    assert!(said.contains(&serde_json::to_string(&expected.join("settings.json")).unwrap()));
}

#[test]
fn state_is_written_only_beneath_the_unicode_profile() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("试用 profile & 100%");
    success(command(Some(&root), "write").output().unwrap());
    assert!(root.join("settings.json").is_file());
    assert!(root.join("ely-doc/settings.json").is_file());
    assert!(root.join("ely-doc/instance.lock").is_file());
    assert!(root.join("ely-doc/instance.door").is_file());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn bad_paths_are_errors_not_a_default_profile() {
    for path in ["", "relative"] {
        let output = command(Some(Path::new(path)), "paths").output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("ELY_DATA_DIR"));
    }
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("not-a-directory");
    std::fs::write(&file, "sentinel").unwrap();
    assert!(
        !command(Some(&file), "write")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(std::fs::read_to_string(file).unwrap(), "sentinel");
}

#[test]
fn clearing_recovery_never_crosses_profiles() {
    let dir = tempfile::tempdir().unwrap();
    for profile in ["a", "b"] {
        let recovery = dir.path().join(profile).join("ely-doc/recovery");
        std::fs::create_dir_all(&recovery).unwrap();
        std::fs::write(recovery.join("kept"), "sentinel").unwrap();
    }
    success(
        command(Some(&dir.path().join("a")), "clear")
            .output()
            .unwrap(),
    );
    assert!(!dir.path().join("a/ely-doc/recovery/kept").exists());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("b/ely-doc/recovery/kept")).unwrap(),
        "sentinel"
    );
}

#[test]
fn separate_profiles_hold_independently_and_hand_to_their_own_copy() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    let first = Holder::start(&a);
    let second = Holder::start(&b);
    for root in [&a, &b] {
        let said = success(command(Some(root), "hand").output().unwrap());
        assert!(said.contains("PROFILE_HANDED"));
    }
    first.finish();
    second.finish();
}

#[test]
#[ignore = "subprocess helper; called explicitly by profile tests"]
fn profile_probe() {
    let mode = std::env::var("ELY_PROFILE_PROBE").unwrap();
    match mode.as_str() {
        "paths" => {
            println!(
                "{}",
                serde_json::to_string(Store::shared().unwrap().path()).unwrap()
            );
            println!(
                "{}",
                serde_json::to_string(&ely_platform::data_dir(AppId::Doc).unwrap()).unwrap()
            );
        }
        "write" => {
            Store::shared()
                .unwrap()
                .set("profile_probe", true.into())
                .unwrap();
            Store::app(AppId::Doc)
                .unwrap()
                .set("profile_probe", true.into())
                .unwrap();
            assert!(matches!(
                ely_platform::instance::hold(AppId::Doc, &[]).unwrap(),
                Start::First(_)
            ));
        }
        "clear" => {
            ely_platform::storage::clear(AppId::Doc, Kind::Recovery).unwrap();
            ely_platform::storage::clear_marked(AppId::Doc).unwrap();
        }
        "hold" => {
            let Start::First(door) = ely_platform::instance::hold(AppId::Doc, &[]).unwrap() else {
                panic!("an independent profile did not take its lock");
            };
            let (sent, heard) = std::sync::mpsc::channel();
            let sent = std::sync::Mutex::new(sent);
            door.serve(move |_| sent.lock().unwrap().send(()).is_ok())
                .unwrap();
            println!("PROFILE_HELD");
            std::io::stdout().flush().unwrap();
            heard.recv_timeout(Duration::from_secs(20)).unwrap();
            let release = ely_platform::data_dir(AppId::Doc)
                .unwrap()
                .parent()
                .unwrap()
                .join("release");
            let deadline = std::time::Instant::now() + Duration::from_secs(20);
            while !release.exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "holder release timed out"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        "hand" => {
            assert!(matches!(
                ely_platform::instance::hold(AppId::Doc, &[]).unwrap(),
                Start::Handed
            ));
            println!("PROFILE_HANDED");
        }
        _ => panic!("unknown profile probe mode"),
    }
}
