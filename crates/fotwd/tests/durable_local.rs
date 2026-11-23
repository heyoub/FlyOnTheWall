//! Local-only sources have no cloud configuration and retain exact restart input.
use fotw_pipeline::replay::{ReplayReader, ReplaySource, ReplayWriter};
use fotw_stt::{
    LocalStreamConfig, Recognized, Recognizer, RecognizerFactory, Source, SttError, SttErrorClass,
};
use fotwd::failover::{
    coverage::CoverageJournal,
    stream::{FailoverStream, LocalResumeConfig, LocalSourceConfig, Phase},
};
use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

struct Factory {
    fail: bool,
    block_drop: Option<Arc<(Mutex<bool>, Condvar)>>,
    entered: Arc<AtomicBool>,
}
struct Model {
    fail: bool,
    block_drop: Option<Arc<(Mutex<bool>, Condvar)>>,
    entered: Arc<AtomicBool>,
}
impl RecognizerFactory for Factory {
    fn model(&self) -> &str {
        "fixture-local"
    }
    fn language(&self) -> Option<&str> {
        Some("en")
    }
    fn create(&self) -> Result<Box<dyn Recognizer>, SttError> {
        Ok(Box::new(Model {
            fail: self.fail,
            block_drop: self.block_drop.clone(),
            entered: self.entered.clone(),
        }))
    }
}
impl Recognizer for Model {
    fn transcribe(&mut self, _: &[f32]) -> Result<Vec<Recognized>, SttError> {
        if self.fail {
            Err(SttError::new(
                SttErrorClass::Server,
                "fixture",
                "synthetic inference failure",
            ))
        } else {
            Ok(vec![Recognized {
                start_ms: 0,
                end_ms: 100,
                text: "local recorded words".into(),
            }])
        }
    }
}
impl Drop for Model {
    fn drop(&mut self) {
        if let Some(release) = &self.block_drop {
            self.entered.store(true, Ordering::Release);
            let mut ready = release.0.lock().unwrap();
            while !*ready {
                ready = release.1.wait(ready).unwrap();
            }
        }
    }
}
struct Release(Arc<(Mutex<bool>, Condvar)>);
impl Drop for Release {
    fn drop(&mut self) {
        *self.0.0.lock().unwrap() = true;
        self.0.1.notify_all();
    }
}
fn factory(fail: bool) -> Arc<dyn RecognizerFactory> {
    Arc::new(Factory {
        fail,
        block_drop: None,
        entered: Arc::new(AtomicBool::new(false)),
    })
}
fn pcm() -> Vec<i16> {
    (0..32000)
        .map(|n| if (n / 480) % 2 == 0 { 1000 } else { 8000 })
        .collect()
}
fn config(local: Arc<dyn RecognizerFactory>) -> LocalSourceConfig {
    let mut config = LocalSourceConfig::new(
        LocalStreamConfig::new("fixture", Source::Mic).with_session_offset_ms(137),
        local,
    );
    config.revision = Some("fixture-v1".into());
    config.stop_timeout = Duration::from_secs(2);
    config
}
fn resume(local: Arc<dyn RecognizerFactory>) -> LocalResumeConfig {
    LocalResumeConfig {
        session_id: "fixture".into(),
        source: Source::Mic,
        local,
        revision: Some("fixture-v1".into()),
        stop_timeout: Duration::from_secs(2),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_local_inference_can_restart_exact_input_with_no_cloud_plan() {
    let dir = tempfile::tempdir().unwrap();
    let (stream, _) = FailoverStream::open_local(dir.path(), config(factory(true)))
        .await
        .unwrap();
    let audio = pcm();
    stream.write(&audio).unwrap();
    assert!(!stream.finish().await);
    assert_eq!(stream.status().phase, Phase::Incomplete);
    let status = stream.status_reader();
    drop(stream);
    let deadline = Instant::now() + Duration::from_secs(2);
    while status.read().worker_running {
        assert!(Instant::now() < deadline);
        tokio::task::yield_now().await;
    }
    let mut reader = ReplayReader::open(dir.path(), "fixture", ReplaySource::Mic).unwrap();
    let mut retained = reader.read_from(137 * 16, 16000).unwrap();
    retained.extend(reader.read_from(137 * 16 + 16000, 16000).unwrap());
    assert_eq!(retained, audio);
    let saved = CoverageJournal::snapshot(dir.path(), "fixture", Source::Mic).unwrap();
    assert_eq!(saved.epochs[0].engine.provider, fotw_stt::local::PROVIDER);
    assert_eq!(saved.epochs[0].reason, None);
    assert_eq!(
        saved.fallback.unwrap().engine.revision.as_deref(),
        Some("fixture-v1")
    );
    let (restarted, _) = FailoverStream::resume_local(dir.path(), resume(factory(false)))
        .await
        .unwrap();
    assert!(restarted.finish().await, "{:?}", restarted.status());
    assert_eq!(
        restarted.status().committed_sample,
        137 * 16 + audio.len() as u64
    );
    assert_eq!(restarted.status().lag_samples, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn completed_data_releases_file_leases_while_native_cleanup_still_excludes_restart() {
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let _release = Release(release.clone());
    let entered = Arc::new(AtomicBool::new(false));
    let dir = tempfile::tempdir().unwrap();
    let model = Arc::new(Factory {
        fail: false,
        block_drop: Some(release.clone()),
        entered: entered.clone(),
    });
    let (stream, _) = FailoverStream::open_local(dir.path(), config(model))
        .await
        .unwrap();
    stream.write(&pcm()).unwrap();
    assert!(stream.finish().await);
    let status = stream.status_reader();
    drop(stream);
    let deadline = Instant::now() + Duration::from_secs(2);
    while !entered.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline);
        tokio::task::yield_now().await;
    }
    assert!(status.read().worker_running);
    assert_eq!(status.read().phase, Phase::Complete);
    let prefix = ReplayWriter::resume_prefix(dir.path(), "fixture", ReplaySource::Mic).unwrap();
    let journal = CoverageJournal::open(dir.path(), "fixture", Source::Mic).unwrap();
    drop(journal);
    drop(prefix);
    assert!(
        FailoverStream::resume_local(dir.path(), resume(factory(false)))
            .await
            .is_err(),
        "native source lease must outlive data publication"
    );
    *release.0.lock().unwrap() = true;
    release.1.notify_all();
    while status.read().worker_running {
        assert!(Instant::now() < deadline);
        tokio::task::yield_now().await;
    }
    let (restarted, _) = FailoverStream::resume_local(dir.path(), resume(factory(false)))
        .await
        .unwrap();
    assert!(restarted.finish().await);
}

struct Initializing {
    release: Arc<(Mutex<bool>, Condvar)>,
    entered: Arc<AtomicBool>,
    inferred: Arc<AtomicBool>,
}
struct Initialized(Arc<AtomicBool>);
impl RecognizerFactory for Initializing {
    fn model(&self) -> &str {
        "fixture-local"
    }
    fn language(&self) -> Option<&str> {
        Some("en")
    }
    fn create(&self) -> Result<Box<dyn Recognizer>, SttError> {
        self.entered.store(true, Ordering::Release);
        let mut ready = self.release.0.lock().unwrap();
        while !*ready {
            ready = self.release.1.wait(ready).unwrap();
        }
        Ok(Box::new(Initialized(self.inferred.clone())))
    }
}
impl Recognizer for Initialized {
    fn transcribe(&mut self, _: &[f32]) -> Result<Vec<Recognized>, SttError> {
        self.0.store(true, Ordering::Release);
        Ok(vec![])
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocked_local_initialization_keeps_input_and_cannot_infer_after_the_stop_deadline() {
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let _release = Release(release.clone());
    let entered = Arc::new(AtomicBool::new(false));
    let inferred = Arc::new(AtomicBool::new(false));
    let local = Arc::new(Initializing {
        release: release.clone(),
        entered: entered.clone(),
        inferred: inferred.clone(),
    });
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = config(local);
    cfg.stop_timeout = Duration::from_millis(30);
    let (stream, _) = tokio::time::timeout(
        Duration::from_secs(1),
        FailoverStream::open_local(dir.path(), cfg),
    )
    .await
    .unwrap()
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !entered.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline);
        tokio::task::yield_now().await;
    }
    let audio = pcm();
    stream.write(&audio).unwrap();
    let mut reader = ReplayReader::open(dir.path(), "fixture", ReplaySource::Mic).unwrap();
    assert_eq!(reader.saved_end().unwrap(), 137 * 16 + audio.len() as u64);
    assert!(
        !tokio::time::timeout(Duration::from_secs(1), stream.finish())
            .await
            .unwrap()
    );
    assert!(stream.status().worker_running);
    let status = stream.status_reader();
    drop(stream);
    *release.0.lock().unwrap() = true;
    release.1.notify_all();
    while status.read().worker_running {
        assert!(Instant::now() < deadline);
        tokio::task::yield_now().await;
    }
    assert!(!inferred.load(Ordering::Acquire));
    assert_eq!(reader.read_from(137 * 16, 16000).unwrap(), audio[..16000]);
    assert_eq!(
        reader.read_from(137 * 16 + 16000, 16000).unwrap(),
        audio[16000..]
    );
}

/// Speech as the endpointer sees it (`fotw-stt`'s `local.rs` tests): a square
/// wave whose level swings 20 dB every 120 ms, then digital silence.
fn utterance(speech_ms: usize, quiet_ms: usize) -> Vec<i16> {
    let speech = (0..speech_ms * 16).map(|i| {
        let amp: i16 = if (i / 1_920) % 2 == 0 { 8_000 } else { 800 };
        if (i / 20) % 2 == 0 { amp } else { -amp }
    });
    speech
        .chain(std::iter::repeat_n(0, quiet_ms * 16))
        .collect()
}

/// A model that holds its first utterance until released.
struct FirstHeld {
    release: Arc<(Mutex<bool>, Condvar)>,
    entered: Arc<AtomicBool>,
}
struct FirstHeldModel {
    release: Arc<(Mutex<bool>, Condvar)>,
    entered: Arc<AtomicBool>,
}
impl RecognizerFactory for FirstHeld {
    fn model(&self) -> &str {
        "fixture-local"
    }
    fn language(&self) -> Option<&str> {
        Some("en")
    }
    fn create(&self) -> Result<Box<dyn Recognizer>, SttError> {
        Ok(Box::new(FirstHeldModel {
            release: self.release.clone(),
            entered: self.entered.clone(),
        }))
    }
}
impl Recognizer for FirstHeldModel {
    fn transcribe(&mut self, pcm: &[f32]) -> Result<Vec<Recognized>, SttError> {
        self.entered.store(true, Ordering::Release);
        let mut ready = self.release.0.lock().unwrap();
        while !*ready {
            ready = self.release.1.wait(ready).unwrap();
        }
        Ok(vec![Recognized {
            start_ms: 0,
            end_ms: pcm.len() as u64 / 16,
            text: "local recorded words".into(),
        }])
    }
}

/// Open a local source whose model holds the first utterance, write that
/// utterance, wait until the model has it, then write a second one.
async fn held_after_more_input(
    dir: &std::path::Path,
) -> (FailoverStream, Arc<(Mutex<bool>, Condvar)>, Vec<i16>) {
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let entered = Arc::new(AtomicBool::new(false));
    let mut cfg = config(Arc::new(FirstHeld {
        release: release.clone(),
        entered: entered.clone(),
    }));
    cfg.stop_timeout = Duration::from_secs(10);
    let (stream, _) = FailoverStream::open_local(dir, cfg).await.unwrap();
    // One utterance, cut at its pause: the model starts on it and is held.
    let mut audio = utterance(500, 900);
    stream.write(&audio).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !entered.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline, "the model never started");
        tokio::task::yield_now().await;
    }
    // More audio arrives while the model works.
    let second = utterance(500, 900);
    stream.write(&second).unwrap();
    audio.extend(second);
    (stream, release, audio)
}

/// Audio written while the model is busy, then Stop, then the model returns:
/// the worker must finish all of the input, not the end it saw before the
/// model started.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn input_written_during_inference_before_stop_is_still_transcribed() {
    let dir = tempfile::tempdir().unwrap();
    let (stream, release, audio) = held_after_more_input(dir.path()).await;
    let _release = Release(release.clone());
    let stream = Arc::new(stream);
    let stopping = tokio::spawn({
        let stream = stream.clone();
        async move { stream.finish().await }
    });
    // Release the model only once Stop has sealed the input.
    let reader = ReplayReader::open(dir.path(), "fixture", ReplaySource::Mic).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while reader.sealed_end().unwrap().is_none() {
        assert!(Instant::now() < deadline, "Stop never sealed the input");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(
        reader.sealed_end().unwrap(),
        Some(137 * 16 + audio.len() as u64)
    );
    // `finish` marks the input closed just after writing the seal, and
    // nothing public shows that flag. Wait a little longer so the old code
    // cannot see the input still open, re-read the end and pass by accident.
    // The fixed code passes either way.
    tokio::time::sleep(Duration::from_millis(250)).await;
    *release.0.lock().unwrap() = true;
    release.1.notify_all();
    let complete = stopping.await.unwrap();
    let status = stream.status();
    assert!(complete, "{status:?}");
    assert_eq!(status.phase, Phase::Complete);
    assert_eq!(status.committed_sample, 137 * 16 + audio.len() as u64);
    assert_eq!(status.lag_samples, 0);
}

/// A source that ends without Stop sealing its input has no final end: the
/// worker must still stop once the model returns, keep every sample, and
/// leave the source to a restart rather than complete it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn input_dropped_during_inference_without_stop_ends_the_worker() {
    let dir = tempfile::tempdir().unwrap();
    let (stream, release, audio) = held_after_more_input(dir.path()).await;
    let _release = Release(release.clone());
    let status = stream.status_reader();
    drop(stream);
    *release.0.lock().unwrap() = true;
    release.1.notify_all();
    let deadline = Instant::now() + Duration::from_secs(5);
    while status.read().worker_running {
        assert!(Instant::now() < deadline, "the worker never stopped");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    // `Drop` already set the phase, so it says nothing about the worker. The
    // checks that do are: the worker stopped, nothing was sealed, every sample
    // was kept, and a restart completes the source.
    let mut reader = ReplayReader::open(dir.path(), "fixture", ReplaySource::Mic).unwrap();
    assert_eq!(reader.sealed_end().unwrap(), None);
    assert_eq!(reader.saved_end().unwrap(), 137 * 16 + audio.len() as u64);
    let mut retained = Vec::new();
    while retained.len() < audio.len() {
        let at = 137 * 16 + retained.len() as u64;
        let want = (audio.len() - retained.len()).min(16_000);
        retained.extend(reader.read_from(at, want).unwrap());
    }
    assert_eq!(retained, audio);
    drop(reader);
    let (restarted, _) = FailoverStream::resume_local(dir.path(), resume(factory(false)))
        .await
        .unwrap();
    assert!(restarted.finish().await, "{:?}", restarted.status());
    assert_eq!(
        restarted.status().committed_sample,
        137 * 16 + audio.len() as u64
    );
}
