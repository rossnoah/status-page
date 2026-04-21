use std::fmt::Write;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(1))
    }

    fn next_f32(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 & 0xFFFF) as f32 / 65536.0
    }
}

pub fn seed_from_str(s: &str) -> u64 {
    let mut h: u64 = 0;
    for b in s.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u64);
    }
    if h == 0 { 1 } else { h }
}

pub fn wobbly_rect(w: f32, h: f32, r: f32, seed: u64, wobble: f32) -> String {
    let mut rng = Rng::new(seed);
    let r = r.min(w / 2.0).min(h / 2.0);
    let j = |rng: &mut Rng, n: f32| -> f32 { n + (rng.next_f32() - 0.5) * wobble };

    let pts: [(f32, f32); 8] = [
        (j(&mut rng, r), 0.0),
        (j(&mut rng, w - r), 0.0),
        (w, j(&mut rng, r)),
        (w, j(&mut rng, h - r)),
        (j(&mut rng, w - r), h),
        (j(&mut rng, r), h),
        (0.0, j(&mut rng, h - r)),
        (0.0, j(&mut rng, r)),
    ];

    let mut d = String::with_capacity(256);
    let _ = write!(d, "M {:.1} {:.1} ", pts[0].0, pts[0].1);
    let _ = write!(d, "L {:.1} {:.1} ", pts[1].0, pts[1].1);
    let _ = write!(d, "Q {w:.1} 0 {:.1} {:.1} ", pts[2].0, pts[2].1);
    let _ = write!(d, "L {:.1} {:.1} ", pts[3].0, pts[3].1);
    let _ = write!(d, "Q {w:.1} {h:.1} {:.1} {:.1} ", pts[4].0, pts[4].1);
    let _ = write!(d, "L {:.1} {:.1} ", pts[5].0, pts[5].1);
    let _ = write!(d, "Q 0 {h:.1} {:.1} {:.1} ", pts[6].0, pts[6].1);
    let _ = write!(d, "L {:.1} {:.1} ", pts[7].0, pts[7].1);
    let _ = write!(d, "Q 0 0 {:.1} {:.1} Z", pts[0].0, pts[0].1);
    d
}

pub fn wobbly_line(width: f32, seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let segs = ((width / 30.0) as usize).max(8);
    let mut d = String::with_capacity(128);
    let _ = write!(d, "M 0 {:.1}", (rng.next_f32() - 0.5));
    for i in 1..=segs {
        let x = width * i as f32 / segs as f32;
        let y = rng.next_f32() - 0.5;
        let _ = write!(d, " L {x:.1} {y:.1}");
    }
    d
}
