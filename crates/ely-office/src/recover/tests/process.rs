use std::{fs, path::Path, process::{Command, Stdio}, time::{Duration, Instant, SystemTime}};

use ely_files::Recovery;

use super::{Keeping, kept};

const PAYLOAD: &[u8] = b"Round20 complete recovery draft: UTF-8 \xe4\xb8\xad\xe6\x96\x87";
const KEY: [u8; 32] = [71; 32];

fn points(folder: std::path::PathBuf, sealed: bool) -> Recovery {
    if sealed { Recovery::sensitive(folder, KEY) } else { Recovery::new(folder) }
}

fn child(dir: &Path, scenario: &str, sealed: bool) -> ! {
    let mut keeping = Keeping::start_in(dir.to_owned()).unwrap();
    let mut record = kept("1-0", false);
    record.sealed = sealed;
    keeping.crashed.push(record);
    points(keeping.folder("1-0"), sealed)
        .write_once(PAYLOAD, None, SystemTime::now()).unwrap().unwrap();
    keeping.save_index().unwrap();
    let pending = keeping.index().unwrap();
    keeping.held = std::mem::take(&mut keeping.crashed);
    #[cfg(windows)]
    let _lock;
    match scenario {
        "before_final" => {}
        "after_index" => keeping.finish_index().unwrap(),
        "after_clean" => keeping.finish(false).unwrap(),
        "after_late" => {
            keeping.finish(false).unwrap();
            pending.publish().unwrap();
        }
        #[cfg(windows)]
        "failed_index" => {
            use std::os::windows::fs::OpenOptionsExt as _;
            // 不共享删除。
            _lock = fs::OpenOptions::new().read(true).share_mode(0x1 | 0x2)
                .open(dir.join("recovery/index.json")).unwrap();
            assert!(keeping.finish(false).is_err(), "the owned lock must refuse replacement");
        }
        _ => panic!("unknown scenario"),
    }
    fs::write(dir.join("ready"), "ready").unwrap();
    loop { std::thread::park(); }
}

fn run(scenario: &str, exact: &str) {
    if let Ok(dir) = std::env::var("ELY_RECOVERY_CHILD_DIR") {
        let sealed = std::env::var("ELY_RECOVERY_CHILD_SEALED").unwrap() == "1";
        child(Path::new(&dir), scenario, sealed);
    }
    for sealed in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", exact, "--nocapture", "--test-threads=1"])
            .env("ELY_RECOVERY_CHILD_DIR", dir.path())
            .env("ELY_RECOVERY_CHILD_SEALED", if sealed { "1" } else { "0" })
            .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let pid = process.id();
        let deadline = Instant::now() + Duration::from_secs(30);
        while !dir.path().join("ready").exists() {
            if let Some(exit) = process.try_wait().unwrap() {
                let output = process.wait_with_output().unwrap();
                panic!("child ended before boundary: {exit}\n{}\n{}",
                    String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            }
            if Instant::now() > deadline {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("owned child {pid} missed recovery boundary");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        process.kill().expect("terminate only this owned test child");
        assert!(!process.wait().unwrap().success(), "a real process termination was injected");
        assert!(!ely_files::alive(pid), "the crash marker must name a dead process");
        let marker = dir.path().join("session.lock").exists();
        assert_eq!(marker, !matches!(scenario, "after_clean" | "after_late"),
            "failed commits cannot remove the crash marker");
        let next = Keeping::start_in(dir.path().to_owned()).expect("start after actual child death");
        assert_eq!(next.crashed.len(), 1, "the committed recovery record must survive {scenario}");
        let recovery = points(next.folder("1-0"), sealed);
        let point = recovery.newest().unwrap().expect("last complete point");
        assert_eq!(point.sealed, sealed);
        if sealed {
            let bytes = fs::read(&point.content).unwrap();
            assert!(!bytes.windows(PAYLOAD.len()).any(|part| part == PAYLOAD));
            assert!(!point.content.with_extension("bin").exists(), "no plaintext recovery sibling");
        }
        let restored = dir.path().join("restored-copy.docx");
        recovery.restore(&point, &restored).expect("restore a copy after process death");
        assert_eq!(fs::read(restored).unwrap(), PAYLOAD);
        println!("RECOVERY_BOUNDARY scenario={scenario} sealed={sealed} pid={pid} recovered=true");
    }
}

macro_rules! boundary {
    ($name:ident, $scenario:literal) => {
        #[test]
        fn $name() { run($scenario, concat!("recover::tests::process::", stringify!($name))); }
    };
}

boundary!(a_process_killed_before_final_commit_restores_the_previous_point, "before_final");
boundary!(a_process_killed_after_final_commit_restores_the_committed_point, "after_index");
boundary!(a_process_killed_after_clean_mark_retains_held_points, "after_clean");
boundary!(a_process_killed_after_late_writer_retains_the_final_index, "after_late");
#[cfg(windows)]
boundary!(a_failed_final_commit_keeps_recovery_after_real_process_death, "failed_index");
