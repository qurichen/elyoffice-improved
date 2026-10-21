use std::time::{Duration, Instant};

use ely_gpui_component::motion;
use gpui::ElementId;

/// The current page and when its fade-in began.
pub(super) struct Fade {
    page: ElementId,
    since: Instant,
}

impl Fade {
    pub(super) fn new(page: ElementId, now: Instant) -> Self {
        Self { page, since: now }
    }

    /// Restarts the fade when the page changes.
    pub(super) fn show(&mut self, page: &ElementId, now: Instant) {
        if self.page != *page {
            self.page = page.clone();
            self.since = now;
        }
    }

    pub(super) fn opacity(&self, now: Instant, length: Duration) -> f32 {
        let elapsed = now.saturating_duration_since(self.since).as_secs_f32();
        let t = (elapsed / length.as_secs_f32().max(f32::EPSILON)).min(1.0);
        0.4 + 0.6 * motion::ease_out_cubic(t)
    }

    pub(super) fn running(&self, now: Instant, length: Duration) -> bool {
        now.saturating_duration_since(self.since) < length
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use gpui::ElementId;

    use super::Fade;

    const LENGTH: Duration = Duration::from_millis(200);

    fn after(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    #[test]
    fn a_fade_rises_from_dim_to_full_and_stays() {
        let t0 = Instant::now();
        let fade = Fade::new("a".into(), t0);

        let first: Vec<f32> = (0..=20)
            .map(|i| fade.opacity(after(t0, i * 10), LENGTH))
            .collect();
        assert_eq!(first[0], 0.4);
        assert!(first.windows(2).all(|pair| pair[1] >= pair[0]), "{first:?}");
        assert_eq!(first[20], 1.0);
        assert!(!fade.running(after(t0, 200), LENGTH));
        assert_eq!(fade.opacity(after(t0, 900), LENGTH), 1.0);
    }

    #[test]
    fn the_same_page_keeps_its_fade() {
        let t0 = Instant::now();
        let mut fade = Fade::new("a".into(), t0);
        let page = ElementId::Name("a".into());

        fade.show(&page, after(t0, 150));

        assert_eq!(fade.opacity(after(t0, 200), LENGTH), 1.0);
    }

    #[test]
    fn a_new_page_restarts_dim() {
        let t0 = Instant::now();
        let mut fade = Fade::new("a".into(), t0);
        assert_eq!(fade.opacity(after(t0, 500), LENGTH), 1.0);

        fade.show(&"b".into(), after(t0, 500));

        assert_eq!(fade.opacity(after(t0, 500), LENGTH), 0.4);
        assert_eq!(fade.opacity(after(t0, 700), LENGTH), 1.0);
    }

    #[test]
    fn reduced_motion_shows_the_page_at_once() {
        let t0 = Instant::now();
        let fade = Fade::new("a".into(), t0);
        let length = Duration::from_millis(1);

        assert_eq!(fade.opacity(after(t0, 1), length), 1.0);
        assert!(!fade.running(after(t0, 1), length));
    }
}
