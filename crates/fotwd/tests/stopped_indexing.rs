use fotw_pipeline::wal::SessionWal;
use fotw_store::{Db, DbKey};
use fotw_stt::{Source, TranscriptSegment};
use fotwd::speech_recovery::SavedSpeechRecovery;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

struct Library {
    root: tempfile::TempDir,
    key: DbKey,
    database: PathBuf,
}

impl Library {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let database = root.path().join("db.sqlite3");
        let key = DbKey::from_bytes([0x5f; 32]);
        drop(Db::open(&database, &key).unwrap());
        Self {
            root,
            key,
            database,
        }
    }
    fn data_root(&self) -> &Path {
        self.root.path()
    }
    fn db(&self) -> Db {
        Db::open(&self.database, &self.key).unwrap()
    }
}

struct Capture {
    dir: PathBuf,
    session: String,
}

/// How far the live path got before the kill.
#[derive(Clone, Copy, Debug)]
enum Kill {
    /// Recording, no Stop request on disk.
    BeforeStop,
    /// Stop checkpoint written, then more drained audio, then killed.
    DuringDrain,
    /// Final Stop checkpoint written; the WAL was never finalized.
    AfterStopRecord,
    /// The WAL was finalized; nothing reached the library.
    AfterFinalize,
    OlderFinalize,
    EmptyAfterFinalize,
}

fn capture(lib: &Library, kill: Kill) -> Capture {
    let mut wal = SessionWal::create(lib.data_root().join("sessions"), 16000, 1).unwrap();
    let dir = wal.dir().to_owned();
    let session = dir.file_name().unwrap().to_str().unwrap().to_owned();
    if !matches!(kill, Kill::EmptyAfterFinalize) {
        wal.write_system(&vec![0.25; 24000]).unwrap();
        wal.write_mic(&vec![0.1; 24000]).unwrap();
    }
    match kill {
        Kill::BeforeStop => {
            wal.checkpoint_capture(1500, false).unwrap();
            drop(wal);
        }
        Kill::DuringDrain => {
            wal.checkpoint_capture(1500, true).unwrap();
            wal.write_system(&vec![0.25; 1600]).unwrap();
            wal.flush().unwrap();
            drop(wal);
        }
        Kill::AfterStopRecord => {
            wal.checkpoint_capture(1500, true).unwrap();
            wal.flush().unwrap();
            drop(wal);
        }
        Kill::AfterFinalize | Kill::EmptyAfterFinalize => {
            wal.checkpoint_capture(1500, true).unwrap();
            drop(wal.finalize_keeping_lease().unwrap());
        }
        Kill::OlderFinalize => {
            wal.checkpoint_capture(1500, true).unwrap();
            wal.finalize().unwrap();
        }
    }
    Capture { dir, session }
}

/// Start-up's index pass, on its own library connection.
fn index(lib: &Library) {
    SavedSpeechRecovery::with_parts(
        lib.db(),
        lib.data_root().to_owned(),
        Arc::new(|_, _| Err("no model in this fixture".into())),
        Duration::from_secs(1),
        Duration::from_secs(1),
    )
    .index_captures()
    .unwrap();
}

fn meetings(lib: &Library) -> usize {
    lib.db().meetings().list(100, 0).unwrap().len()
}

fn segment(session: &str, id: &str, text: &str) -> TranscriptSegment {
    let mut s = TranscriptSegment::new(session, Source::System, "fixture", "synthetic");
    s.id = id.into();
    s.text = text.into();
    s.start_ms = 100;
    s.end_ms = 900;
    s.is_final = true;
    s
}

fn library_text(lib: &Library, session: &str) -> Vec<String> {
    let db = lib.db();
    db.session_note_meeting(session)
        .unwrap()
        .map(|id| {
            db.export_meeting(&id)
                .unwrap()
                .segments
                .into_iter()
                .map(|s| s.text)
                .collect()
        })
        .unwrap_or_default()
}

fn snapshot(dir: &Path) -> std::collections::BTreeMap<std::ffi::OsString, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), std::fs::read(entry.path()).unwrap())
        })
        .collect()
}

#[test]
fn a_kill_after_the_live_transcript_reached_stt_jsonl_does_not_delete_it() {
    let lib = Library::new();
    let c = capture(&lib, Kill::AfterFinalize);
    let line = "words preserved before the kill";
    fotwd::session::append_segments(&c.dir, &[segment(&c.session, "only", line)]).unwrap();
    let bytes = snapshot(&c.dir);
    for _ in 0..2 {
        index(&lib);
        let mut db = lib.db();
        assert_eq!(meetings(&lib), 1);
        let id = db.session_note_meeting(&c.session).unwrap().unwrap();
        assert_eq!(db.meetings().get(&id).unwrap().state, "interrupted");
        assert_eq!(library_text(&lib, &c.session), [line]);
        assert!(fotwd::retention::resume_promotions(&mut db, lib.data_root()).is_empty());
        assert_eq!(snapshot(&c.dir), bytes);
    }
}

#[test]
fn stopped_captures_without_a_transcript_are_interrupted_when_admitted() {
    for kill in [
        Kill::BeforeStop,
        Kill::DuringDrain,
        Kill::AfterStopRecord,
        Kill::AfterFinalize,
        Kill::EmptyAfterFinalize,
    ] {
        let lib = Library::new();
        let c = capture(&lib, kill);
        index(&lib);
        let mut db = lib.db();
        let id = db.session_note_meeting(&c.session).unwrap().unwrap();
        assert_eq!(
            db.meetings().get(&id).unwrap().state,
            "interrupted",
            "{kill:?}"
        );
        assert!(c.dir.join("system.pcm").is_file());
    }
}

#[test]
fn older_finisher_must_still_have_its_log_destination() {
    let lib = Library::new();
    let c = capture(&lib, Kill::OlderFinalize);
    index(&lib);
    assert!(lib.db().session_note_meeting(&c.session).unwrap().is_none());
    let late = segment(&c.session, "old", "words from the older finisher");
    fotwd::session::append_segments(&c.dir, &[late]).unwrap();
    index(&lib);
    assert!(lib.db().session_note_meeting(&c.session).unwrap().is_none());
    assert!(
        std::fs::read_to_string(c.dir.join("stt.jsonl"))
            .unwrap()
            .contains("words from the older finisher")
    );
}

#[test]
fn a_log_read_error_is_reported_without_importing_a_valid_prefix() {
    let valid = br#"{"seq":0,"t0_ms":1,"t1_ms":2,"text":"fixture","audio_byte_offset":0}"#;
    for suffix in [b"\n{broken}\n".as_slice(), b"\n\xff\n", b""] {
        let lib = Library::new();
        let c = capture(&lib, Kill::AfterFinalize);
        let path = c.dir.join("stt.jsonl");
        let mut bytes = valid.to_vec();
        bytes.extend_from_slice(suffix);
        std::fs::write(&path, &bytes).unwrap();
        index(&lib);
        assert_eq!(meetings(&lib), 0);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }

    let lib = Library::new();
    let c = capture(&lib, Kill::AfterFinalize);
    let unknown = concat!(
        r#"{"seq":0,"t0_ms":1,"t1_ms":2,"text":"fixture","audio_byte_offset":0,"unknown":true}"#,
        "\n"
    )
    .as_bytes();
    let path = c.dir.join("stt.jsonl");
    std::fs::write(&path, unknown).unwrap();
    index(&lib);
    assert_eq!(meetings(&lib), 0);
    assert_eq!(std::fs::read(&path).unwrap(), unknown);

    let log = c.dir.join("stt.jsonl");
    std::fs::remove_file(&log).unwrap();
    std::fs::create_dir(&log).unwrap();
    let evidence = log.join("original");
    std::fs::write(&evidence, b"synthetic transcript evidence").unwrap();
    index(&lib);
    assert_eq!(meetings(&lib), 0);
    assert_eq!(
        std::fs::read(evidence).unwrap(),
        b"synthetic transcript evidence"
    );
}

#[test]
fn an_oversized_legacy_record_is_retained_and_does_not_block_other_captures() {
    assert_oversized_log_retained(256 * 1024, 1);
}

#[test]
fn an_oversized_legacy_log_is_retained_and_does_not_block_other_captures() {
    assert_oversized_log_retained(64 * 1024, 130);
}

fn assert_oversized_log_retained(text_bytes: usize, records: usize) {
    use std::io::Write;
    let lib = Library::new();
    let oversized = capture(&lib, Kill::AfterFinalize);
    let next = capture(&lib, Kill::AfterFinalize);
    let path = oversized.dir.join("stt.jsonl");
    let mut log = std::fs::File::create(&path).unwrap();
    let text = "x".repeat(text_bytes);
    for seq in 0..records {
        writeln!(
            log,
            r#"{{"seq":{seq},"t0_ms":0,"t1_ms":100,"text":"{text}","audio_byte_offset":0}}"#
        )
        .unwrap();
    }
    drop(log);
    let bytes = std::fs::read(&path).unwrap();

    index(&lib);

    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let db = lib.db();
    assert!(db.session_note_meeting(&next.session).unwrap().is_some());
    assert!(
        db.session_note_meeting(&oversized.session)
            .unwrap()
            .is_none(),
        "imported an oversized legacy log"
    );
}
