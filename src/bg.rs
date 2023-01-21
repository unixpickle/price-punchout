use rand::{
    distributions::{Distribution, Uniform},
    Rng,
};

const NUM_PIECES: usize = 40;
const MIN_X_DIST: f32 = 1.0 / 100.0;
const MIN_RADIUS: f32 = 0.02;
const MAX_RADIUS: f32 = 0.1;
const TEXT_TO_RADIUS_SIZE: f32 = 1.3;

pub struct Background {
    pub html: String,
    pub css: String,
}

impl Background {
    pub fn sample<R: Rng>(rng: &mut R) -> Background {
        let mut pieces = Vec::<PieceTrajectory>::new();
        while pieces.len() < NUM_PIECES {
            let piece = PieceTrajectory::sample(rng);
            let closest = pieces
                .iter()
                .map(|p1| (p1.x - piece.x).abs())
                .fold(f32::INFINITY, |a, b| a.min(b));
            if closest > MIN_X_DIST {
                pieces.push(piece);
            }
        }
        pieces.sort_by(|x, y| x.radius.partial_cmp(&y.radius).unwrap());
        let fragments = pieces
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let name = format!("background-piece-{}", i);
                (x.html_fragment(&name), x.css_fragment(&name))
            })
            .collect::<Vec<_>>();
        Background {
            html: fragments
                .iter()
                .map(|(x, _)| x.clone())
                .collect::<Vec<_>>()
                .join("")
                .replace("\n", ""),
            css: fragments
                .into_iter()
                .map(|(_, x)| x)
                .collect::<Vec<_>>()
                .join("")
                .replace("\n", ""),
        }
    }
}

struct PieceTrajectory {
    radius: f32,
    x: f32,
    start_y: f32,
    duration: f32,
}

impl PieceTrajectory {
    fn sample<R: Rng>(rng: &mut R) -> PieceTrajectory {
        let radius = Uniform::new(MIN_RADIUS, MAX_RADIUS).sample(rng);
        let x = Uniform::new(0.0, 1.0).sample(rng) - radius / 2.0;
        let start_y = Uniform::new(0.0, 1.0).sample(rng) - radius / 2.0;
        let duration = 3.0 / radius;
        PieceTrajectory {
            radius,
            x,
            start_y,
            duration,
        }
    }

    fn html_fragment(&self, name: &str) -> String {
        format!(
            "
<div class=\"background-piece\" id=\"{name}\"></div>
        ",
            name = name
        )
        .trim()
        .to_owned()
    }

    fn css_fragment(&self, name: &str) -> String {
        let left_x = self.x - self.radius;
        let bottom_y = 1.0;
        let top_y = -self.radius * 2.0;

        let time_to_bottom = bottom_y - self.start_y;
        let time_from_top = self.start_y - top_y;
        let mid_frac = time_to_bottom / (time_to_bottom + time_from_top);

        format!(
            "
#{name} {{
    width: {size:.05}em;
    height: {size:.05}em;
    left: {left_x:.05}em;
    opacity: {opacity:.05};
    animation-duration: {duration:.02}s;
    animation-iteration-count: infinite;
    animation-name: {name}-keyframes;
    animation-timing-function: linear;
    animation-delay: {delay:.02}s;
}}

#{name}::before {{
    font-size: {text_size:.05}em;
}}

@keyframes {name}-keyframes {{
    from {{
        top: {top_y:.05}em;
    }}
    to {{
        top: {bottom_y:.05}em;
    }}
}}
        ",
            left_x = left_x,
            duration = self.duration,
            size = self.radius * 2.0,
            text_size = self.radius * TEXT_TO_RADIUS_SIZE,
            opacity = self.radius / MAX_RADIUS,
            name = name,
            delay = -self.duration * mid_frac,
            top_y = top_y,
            bottom_y = bottom_y
        )
        .trim()
        .to_owned()
    }
}
