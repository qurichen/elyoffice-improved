//! How two slides meet in each transition Ely offers.

/// Where a slide shows: x0, y0, x1, y1 as shares.
pub type Area = (f32, f32, f32, f32);

const WHOLE: Area = (0.0, 0.0, 1.0, 1.0);

/// One frame: offsets, shown areas and black over all.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub from_shift: (f32, f32),
    pub to_shift: (f32, f32),
    pub to_opacity: f32,
    /// Where the new slide shows, over the old.
    pub to_areas: Vec<Area>,
    pub black: f32,
    /// The old slide lies over the new one.
    pub old_on_top: bool,
}

/// Dissolve's squares a side.
const GRID: usize = 12;

/// The new slide alone, in place.
pub fn done() -> Frame {
    Frame {
        from_shift: (0.0, 0.0),
        to_shift: (0.0, 0.0),
        to_opacity: 1.0,
        to_areas: vec![WHOLE],
        black: 0.0,
        old_on_top: false,
    }
}

/// The frame of `preset` at progress `p`, 0 to 1.
pub fn frame(preset: &str, p: f32) -> Frame {
    let p = p.clamp(0.0, 1.0);
    let base = done();
    match preset {
        "fade-smoothly" => Frame {
            to_opacity: p,
            ..base
        },
        // Down to black, then up from it.
        "fade-through-black" => Frame {
            to_opacity: if p < 0.5 { 0.0 } else { 1.0 },
            black: 1.0 - (2.0 * p - 1.0).abs(),
            ..base
        },
        "push-left" => Frame {
            from_shift: (-p, 0.0),
            to_shift: (1.0 - p, 0.0),
            ..base
        },
        "push-up" => Frame {
            from_shift: (0.0, -p),
            to_shift: (0.0, 1.0 - p),
            ..base
        },
        "cover-left" => Frame {
            to_shift: (1.0 - p, 0.0),
            ..base
        },
        // The old slide slides away; the new one waits beneath.
        "uncover-left" => Frame {
            from_shift: (-p, 0.0),
            old_on_top: true,
            ..base
        },
        "wipe-right" => Frame {
            to_areas: vec![(0.0, 0.0, p, 1.0)],
            ..base
        },
        "split-vertical-out" => Frame {
            to_areas: vec![(0.5 - p / 2.0, 0.0, 0.5 + p / 2.0, 1.0)],
            ..base
        },
        "dissolve" => Frame {
            to_areas: dissolved(p),
            ..base
        },
        // No transition, and ones the preview does not draw.
        _ => base,
    }
}

/// The new slide's squares shown at `p`, fixed scatter.
fn dissolved(p: f32) -> Vec<Area> {
    let count = GRID * GRID;
    let shown = (p * count as f32).round() as usize;
    let side = 1.0 / GRID as f32;
    // A stride coprime with the count visits every square once.
    (0..shown)
        .map(|n| (n * 89) % count)
        .map(|cell| {
            let (x, y) = ((cell % GRID) as f32 * side, (cell / GRID) as f32 * side);
            (x, y, x + side, y + side)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_offered_transition_ends_on_the_new_slide_alone() {
        for preset in ely_lok::show::TRANSITIONS {
            let end = frame(preset, 1.0);
            assert_eq!(
                (end.to_shift, end.to_opacity, end.black),
                ((0.0, 0.0), 1.0, 0.0),
                "{preset}"
            );
            let covered: f32 = end.to_areas.iter().map(|a| (a.2 - a.0) * (a.3 - a.1)).sum();
            assert!((covered - 1.0).abs() < 0.001, "{preset} covers {covered}");
        }
    }

    #[test]
    fn only_uncover_lays_the_old_slide_on_top() {
        for preset in ely_lok::show::TRANSITIONS {
            assert_eq!(
                frame(preset, 0.5).old_on_top,
                preset == "uncover-left",
                "{preset}"
            );
        }
    }

    #[test]
    fn halfway_frames_mix_both_slides() {
        assert_eq!(frame("push-left", 0.5).from_shift, (-0.5, 0.0));
        assert_eq!(frame("wipe-right", 0.5).to_areas, [(0.0, 0.0, 0.5, 1.0)]);
        assert_eq!(frame("fade-through-black", 0.5).black, 1.0);
        let half = frame("dissolve", 0.5).to_areas;
        assert_eq!(half.len(), GRID * GRID / 2);
        let mut cells: Vec<_> = frame("dissolve", 1.0)
            .to_areas
            .iter()
            .map(|a| ((a.0 * 100.0) as i32, (a.1 * 100.0) as i32))
            .collect();
        cells.sort();
        cells.dedup();
        assert_eq!(cells.len(), GRID * GRID, "no square twice");
    }
}
