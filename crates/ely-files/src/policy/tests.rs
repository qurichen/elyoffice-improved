use super::*;

fn at(base: Instant, secs: u64) -> Instant {
    base + Duration::from_secs(secs)
}

#[test]
fn explicit_never_saves_by_itself() {
    let t0 = Instant::now();
    let mut engine = PolicyEngine::new(Policy::Explicit, true);
    engine.edit(t0);
    for n in 0..10 {
        assert_eq!(engine.tick(at(t0, n)), Action::Nothing);
    }
    assert!(!engine.autosaving());
}

#[test]
fn autosave_fires_a_second_after_typing_stops() {
    let t0 = Instant::now();
    let mut engine = PolicyEngine::new(Policy::AutoSave, true);
    engine.edit(t0);
    assert_eq!(engine.tick(t0), Action::Nothing);
    assert_eq!(engine.tick(at(t0, 1)), Action::SaveDue(1));
    engine.save_finished(1, true);
    assert_eq!(engine.tick(at(t0, 2)), Action::Nothing);
    engine.edit(at(t0, 3));
    assert_eq!(engine.tick(at(t0, 4)), Action::SaveDue(2));
}

#[test]
fn the_debounce_restarts_per_edit() {
    let t0 = Instant::now();
    let mut engine = PolicyEngine::new(Policy::MarkAutoSave, true);
    engine.edit(t0);
    engine.edit(t0 + Duration::from_millis(700));
    assert_eq!(engine.tick(at(t0, 1)), Action::Nothing);
    assert_eq!(
        engine.tick(t0 + Duration::from_millis(1800)),
        Action::SaveDue(2)
    );
}

#[test]
fn a_save_leaves_newer_edits_their_due() {
    let t0 = Instant::now();
    let mut engine = PolicyEngine::new(Policy::MarkAutoSave, true);
    engine.edit(t0);
    assert_eq!(engine.tick(at(t0, 1)), Action::SaveDue(1));
    engine.edit(t0 + Duration::from_millis(1200));
    engine.save_finished(1, true);
    assert_eq!(
        engine.tick(t0 + Duration::from_millis(2100)),
        Action::Nothing
    );
    assert_eq!(
        engine.tick(t0 + Duration::from_millis(2300)),
        Action::SaveDue(2),
        "the newer edit gets its own second"
    );
    engine.save_finished(2, true);
    assert_eq!(engine.tick(at(t0, 4)), Action::Nothing);
}

#[test]
fn a_failed_autosave_stays_dirty() {
    let t0 = Instant::now();
    let mut engine = PolicyEngine::new(Policy::AutoSave, true);
    engine.edit(t0);
    assert_eq!(engine.tick(at(t0, 1)), Action::SaveDue(1));
    engine.save_finished(1, false);
    assert_eq!(engine.tick(at(t0, 2)), Action::SaveDue(1));
}

#[test]
fn an_external_change_stops_autosave_for_good() {
    let t0 = Instant::now();
    let mut engine = PolicyEngine::new(Policy::AutoSave, true);
    assert_eq!(engine.external_modified(), Action::AutoSaveStopped);
    assert!(!engine.autosaving());
    engine.edit(t0);
    assert_eq!(engine.tick(at(t0, 10)), Action::Nothing);
    assert_eq!(engine.external_modified(), Action::Nothing, "reported once");
}

#[test]
fn a_compat_risk_stops_autosave_too() {
    let t0 = Instant::now();
    let mut engine = PolicyEngine::new(Policy::MarkAutoSave, true);
    assert_eq!(engine.compat_risk(), Action::AutoSaveStopped);
    engine.edit(t0);
    assert_eq!(engine.tick(at(t0, 10)), Action::Nothing);
}

#[test]
fn mark_without_a_path_recovers_until_first_save() {
    let t0 = Instant::now();
    let mut engine = PolicyEngine::new(Policy::MarkAutoSave, false);
    engine.edit(t0);
    for n in 0..10 {
        assert_eq!(engine.tick(at(t0, n)), Action::Nothing);
    }
    engine.edit(at(t0, 5));
    engine.set_has_path();
    assert_eq!(
        engine.tick(at(t0, 6)),
        Action::SaveDue(2),
        "the pathless edits wait for a path"
    );
}

#[test]
fn pathless_edits_during_the_first_save_get_scheduled() {
    let t0 = Instant::now();
    let mut engine = PolicyEngine::new(Policy::MarkAutoSave, false);
    engine.edit(t0);
    engine.edit(t0 + Duration::from_millis(500));
    engine.save_finished(1, true);
    engine.set_has_path();
    assert_eq!(
        engine.tick(t0 + Duration::from_millis(1400)),
        Action::Nothing
    );
    assert_eq!(
        engine.tick(t0 + Duration::from_millis(1600)),
        Action::SaveDue(2)
    );
}

#[test]
fn explicit_ignores_the_stop_triggers() {
    let mut engine = PolicyEngine::new(Policy::Explicit, true);
    assert_eq!(engine.external_modified(), Action::Nothing);
    assert_eq!(engine.compat_risk(), Action::Nothing);
}
