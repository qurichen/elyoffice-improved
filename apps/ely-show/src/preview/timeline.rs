//! When each effect runs, and how its object looks meanwhile.

use ely_lok::show::Effect;

/// How an object looks at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub visible: bool,
    pub opacity: f32,
    /// Offset as a share of the slide's width and height.
    pub shift: (f32, f32),
    /// Scale about the object's centre.
    pub scale: f32,
    /// Share of the object shown, from its bottom up.
    pub reveal: f32,
}

impl Look {
    pub const SHOWN: Look = Look {
        visible: true,
        opacity: 1.0,
        shift: (0.0, 0.0),
        scale: 1.0,
        reveal: 1.0,
    };
    pub const HIDDEN: Look = Look {
        visible: false,
        ..Look::SHOWN
    };
}

/// One effect placed on the clock, in seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub hash: String,
    pub preset: String,
    pub begin: f64,
    pub end: f64,
}

/// Places the effects back to back, as a preview plays.
pub fn schedule(effects: &[Effect]) -> Vec<Run> {
    let (mut runs, mut last_begin, mut last_end) = (Vec::new(), 0.0f64, 0.0f64);
    let mut end_of_all = 0.0f64;
    for effect in effects {
        // A click waits for all before; previews click at once.
        let anchor = match effect.start.as_str() {
            "with" => last_begin,
            "after" => last_end,
            _ => end_of_all,
        };
        let begin = anchor + effect.delay;
        let end = begin + effect.duration.max(0.0);
        runs.push(Run {
            hash: effect.hash.clone(),
            preset: effect.preset.clone(),
            begin,
            end,
        });
        (last_begin, last_end) = (begin, end);
        end_of_all = end_of_all.max(end);
    }
    runs
}

/// The ease the kernel's presets mostly use.
fn ease(p: f32) -> f32 {
    p * p * (3.0 - 2.0 * p)
}

/// An entrance at progress `p`: 0 hidden, 1 in place.
fn entrance(kind: &str, p: f32, top: f32) -> Look {
    let p = ease(p.clamp(0.0, 1.0));
    match kind {
        "fade-in" => Look {
            opacity: p,
            ..Look::SHOWN
        },
        // From below the slide's bottom edge.
        "fly-in" => Look {
            shift: (0.0, (1.0 - p) * (1.0 - top)),
            ..Look::SHOWN
        },
        "wipe" => Look {
            reveal: p,
            ..Look::SHOWN
        },
        "zoom" => Look {
            scale: p,
            ..Look::SHOWN
        },
        "float" => Look {
            opacity: p,
            shift: (0.0, (1.0 - p) * 0.1),
            ..Look::SHOWN
        },
        // Appear, and effects the preview does not draw.
        _ => {
            if p > 0.0 {
                Look::SHOWN
            } else {
                Look::HIDDEN
            }
        }
    }
}

/// How `run`'s object looks at second `t`, once started.
pub fn look(run: &Run, t: f64, top: f32) -> Option<Look> {
    if t < run.begin {
        return None;
    }
    let span = run.end - run.begin;
    let p = if span <= 0.0 {
        1.0
    } else {
        ((t - run.begin) / span).min(1.0) as f32
    };
    let (class, kind) = run
        .preset
        .strip_prefix("ooo-")
        .and_then(|rest| rest.split_once('-'))
        .unwrap_or(("other", ""));
    Some(match (class, kind) {
        ("entrance", kind) => entrance(kind, if span <= 0.0 { 1.0 } else { p }, top),
        ("exit", kind) => {
            let mirrored = kind.replace("-out", "-in").replace("disappear", "appear");
            let gone = entrance(&mirrored, 1.0 - p, top);
            if p >= 1.0 { Look::HIDDEN } else { gone }
        }
        ("emphasis", "grow-and-shrink") => Look {
            scale: 1.0 + 0.5 * (1.0 - (2.0 * p - 1.0).abs()),
            ..Look::SHOWN
        },
        ("emphasis", "blink") => {
            if p < 0.5 && p > 0.0 {
                Look::HIDDEN
            } else {
                Look::SHOWN
            }
        }
        _ => Look::SHOWN,
    })
}

/// The object's look at `t`, from its last started run.
pub fn object_look(runs: &[Run], hash: &str, visible: bool, t: f64, top: f32) -> Look {
    let ran = runs
        .iter()
        .rev()
        .filter(|run| run.hash == hash)
        .find_map(|run| look(run, t, top));
    ran.unwrap_or(if visible { Look::SHOWN } else { Look::HIDDEN })
}

/// When the last effect ends.
pub fn length(runs: &[Run]) -> f64 {
    runs.iter().map(|run| run.end).fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(preset: &str, start: &str, duration: f64, delay: f64) -> Effect {
        Effect {
            preset: preset.into(),
            label: String::new(),
            class: String::new(),
            target: Some(0),
            hash: preset.into(),
            start: start.into(),
            duration,
            delay,
        }
    }

    #[test]
    fn clicks_wait_with_joins_and_after_follows() {
        let runs = schedule(&[
            effect("ooo-entrance-fade-in", "click", 2.0, 0.0),
            effect("ooo-entrance-fly-in", "with", 1.0, 0.5),
            effect("ooo-entrance-wipe", "after", 1.0, 0.0),
            effect("ooo-entrance-zoom", "click", 1.0, 0.0),
        ]);
        let times: Vec<(f64, f64)> = runs.iter().map(|r| (r.begin, r.end)).collect();
        assert_eq!(times, [(0.0, 2.0), (0.5, 1.5), (1.5, 2.5), (2.5, 3.5)]);
        assert_eq!(length(&runs), 3.5);
    }

    #[test]
    fn entrances_hide_before_and_rest_in_place_after() {
        let run = &schedule(&[effect("ooo-entrance-fade-in", "click", 2.0, 1.0)])[0];
        let at = |t| object_look(std::slice::from_ref(run), &run.hash, false, t, 0.3);
        assert_eq!(at(0.5), Look::HIDDEN);
        assert!((at(2.0).opacity - 0.5).abs() < 0.01);
        assert_eq!(at(4.0), Look::SHOWN);
    }

    #[test]
    fn exits_end_hidden_and_emphasis_returns() {
        let runs = schedule(&[
            effect("ooo-exit-fly-out", "click", 1.0, 0.0),
            effect("ooo-emphasis-grow-and-shrink", "click", 2.0, 0.0),
        ]);
        let out = |t| object_look(&runs[..1], &runs[0].hash, true, t, 0.5);
        assert_eq!(out(0.0), Look::SHOWN);
        assert!(out(0.5).shift.1 > 0.0, "on its way down");
        assert_eq!(out(1.0), Look::HIDDEN);
        let grow = |t| object_look(&runs[1..], &runs[1].hash, true, t, 0.5).scale;
        assert!((grow(2.0) - 1.5).abs() < 0.01, "biggest halfway");
        assert!((grow(3.0) - 1.0).abs() < 0.01, "back after");
    }
}
