//! Sets of pixels: what a diagram's lines, regions and arrowheads cover on
//! the grid, gathered and then drawn at once.

use std::collections::HashSet;

use limu::scene::{Point, Polyline};

use super::draw::Ink;
use crate::paint::Pen;

/// A pixel of the pen's grid, by its top left corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct Pixel {
    pub x: i32,
    pub y: i32,
}

/// A set of pixels, gathered and then drawn at once, so that a pixel two
/// lines cross is drawn once, and a color that is not opaque comes out even.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pixels(HashSet<Pixel>);

impl Pixels {
    pub fn add(&mut self, x: i32, y: i32) {
        self.0.insert(Pixel { x, y });
    }

    pub fn has(&self, x: i32, y: i32) -> bool {
        self.0.contains(&Pixel { x, y })
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn extend(&mut self, other: &Pixels) {
        self.0.extend(other.0.iter().copied());
    }

    /// Draws the pixels in an ink, in runs along their rows.
    pub fn draw(&self, pen: &mut Pen, ink: Ink) {
        if self.0.is_empty() || ink.alpha == 0 {
            return;
        }
        let mut all: Vec<Pixel> = self.0.iter().copied().collect();
        all.sort_by_key(|p| (p.y, p.x));
        let mut i = 0;
        while i < all.len() {
            let mut j = i + 1;
            while j < all.len() && all[j].y == all[i].y && all[j].x == all[j - 1].x + 1 {
                j += 1;
            }
            ink.rect(pen, all[i].x, all[i].y, (j - i) as i32, 1);
            i = j;
        }
    }

    /// The pixels next to a pixel outside the set, across or down: its
    /// outline, a pixel wide, as QuickDraw frames a region.
    pub fn border(&self) -> Pixels {
        Pixels(
            self.0
                .iter()
                .copied()
                .filter(|p| {
                    !self.has(p.x - 1, p.y)
                        || !self.has(p.x + 1, p.y)
                        || !self.has(p.x, p.y - 1)
                        || !self.has(p.x, p.y + 1)
                })
                .collect(),
        )
    }

    /// The pixels that are not in other.
    pub fn without(&self, other: &Pixels) -> Pixels {
        Pixels(self.0.difference(&other.0).copied().collect())
    }
}

/// The pixels whose centers are inside polygons, by the nonzero rule, as SVG
/// fills.
pub(crate) fn fill(polys: &[Polyline]) -> Pixels {
    let mut px = Pixels::default();
    let (mut top, mut bottom) = (f64::INFINITY, f64::NEG_INFINITY);
    for l in polys {
        for p in &l.points {
            top = top.min(p.y);
            bottom = bottom.max(p.y);
        }
    }
    if top > bottom {
        return px;
    }
    let mut xs: Vec<(f64, i32)> = Vec::new();
    let mut y = top.floor() as i32;
    while f64::from(y) < bottom {
        let yc = f64::from(y) + 0.5;
        xs.clear();
        for l in polys {
            let n = l.points.len();
            for i in 0..n {
                let (a, b) = (l.points[i], l.points[(i + 1) % n]);
                if (a.y <= yc) == (b.y <= yc) {
                    continue;
                }
                let dir = if b.y < a.y { -1 } else { 1 };
                xs.push((a.x + (yc - a.y) * (b.x - a.x) / (b.y - a.y), dir));
            }
        }
        xs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut wind = 0;
        let mut from = 0.0;
        for &(x, dir) in &xs {
            let was = wind;
            wind += dir;
            if was == 0 && wind != 0 {
                from = x;
            } else if was != 0 && wind == 0 {
                let mut px_x = (from - 0.5).ceil() as i32;
                while f64::from(px_x) + 0.5 < x {
                    px.add(px_x, y);
                    px_x += 1;
                }
            }
        }
        y += 1;
    }
    px
}

/// How lines go on pixels: as wide as a brush, in dashes of a pattern, and
/// from a point of the scene to the pixel it falls in, by place.
pub(crate) struct Stroke<F: Fn(Point) -> Pixel> {
    pub width: i32,
    /// Lengths on and off, in pixels; none is solid.
    pub dash: Vec<i32>,
    pub at: F,
}

impl<F: Fn(Point) -> Pixel> Stroke<F> {
    /// Puts polylines on pixels, one pixel after the other as Bresenham
    /// steps, the pattern of dashes running on from one line to the next.
    pub fn lines(&self, px: &mut Pixels, lines: &[Polyline]) {
        let pattern: i32 = self.dash.iter().sum::<i32>().max(1);
        let mut step = 0;
        let mut put = |x: i32, y: i32| {
            if !self.dash.is_empty() {
                let (mut on, mut at) = (true, step % pattern);
                for &d in &self.dash {
                    if at < d {
                        break;
                    }
                    at -= d;
                    on = !on;
                }
                step += 1;
                if !on {
                    return;
                }
            }
            let lo = -(self.width - 1) / 2;
            for dy in lo..lo + self.width {
                for dx in lo..lo + self.width {
                    px.add(x + dx, y + dy);
                }
            }
        };
        for l in lines {
            let mut points = l.points.clone();
            if l.closed && !points.is_empty() {
                points.push(points[0]);
            }
            let mut last = Pixel { x: 0, y: 0 };
            for (i, &p) in points.iter().enumerate() {
                let q = (self.at)(p);
                if i == 0 {
                    put(q.x, q.y);
                } else {
                    bresenham(last, q, &mut put);
                }
                last = q;
            }
        }
    }
}

/// Calls put for each pixel of the line from a to b, but a.
fn bresenham(a: Pixel, b: Pixel, put: &mut impl FnMut(i32, i32)) {
    let (dx, dy) = ((b.x - a.x).abs(), -(b.y - a.y).abs());
    let (sx, sy) = ((b.x - a.x).signum(), (b.y - a.y).signum());
    let mut err = dx + dy;
    let (mut x, mut y) = (a.x, a.y);
    while x != b.x || y != b.y {
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
        put(x, y);
    }
}

/// The pixel a point of a line falls in.
pub(crate) fn floor_at(p: Point) -> Pixel {
    Pixel {
        x: p.x.floor() as i32,
        y: p.y.floor() as i32,
    }
}

/// Where a point of a line inside a box falls: on the pixel by it toward the
/// box's middle, so that a line drawn at the box's sides lands on the pixels
/// of its frame, and lines alike from either side are alike from it.
pub(crate) fn inward(b: limu::scene::Rect) -> impl Fn(Point) -> Pixel {
    let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
    let at = |v: f64, c: f64| {
        if v <= c {
            v.floor() as i32
        } else {
            v.ceil() as i32 - 1
        }
    };
    move |p: Point| Pixel {
        x: at(p.x, cx),
        y: at(p.y, cy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use limu::scene::Path;

    fn line(points: &[(f64, f64)]) -> Vec<Polyline> {
        vec![Polyline {
            points: points.iter().map(|&(x, y)| Point::new(x, y)).collect(),
            closed: false,
        }]
    }

    // A box filled is the pixels within it, and its border the ring of them
    // at its edge.
    #[test]
    fn fill_box() {
        let mut square = Path::new();
        square.polygon(&[(2., 3.), (7., 3.), (7., 6.), (2., 6.)]);
        let px = fill(&square.flatten(0.25));
        assert_eq!(px.len(), 15);
        assert!(px.has(2, 3) && px.has(6, 5) && !px.has(7, 3) && !px.has(2, 6));
        let b = px.border();
        assert_eq!(b.len(), 12);
        assert!(b.has(2, 4) && !b.has(3, 4));
    }

    // A line covers each pixel from its start to its end once,
    // eight-connected, and dashes leave the pixels of their gaps.
    #[test]
    fn lines() {
        let mut px = Pixels::default();
        let solid = Stroke {
            width: 1,
            dash: vec![],
            at: floor_at,
        };
        solid.lines(&mut px, &line(&[(0., 0.), (6., 3.)]));
        assert_eq!(px.len(), 7);
        assert!(px.has(0, 0) && px.has(6, 3));

        let mut px = Pixels::default();
        let dashed = Stroke {
            width: 1,
            dash: vec![2, 1],
            at: floor_at,
        };
        dashed.lines(&mut px, &line(&[(0., 0.), (8., 0.)]));
        for x in 0..9 {
            assert_eq!(px.has(x, 0), x % 3 != 2, "dashes: {px:?}");
        }
    }
}
