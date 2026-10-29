//! Drawing a scene's items in the Platinum look, on whole pixels with a
//! [`Pen`].

use gpui::{Pixels as Px, Point as GpuiPoint};
use limu::scene::style::{Color, Resolved};
use limu::scene::text::Face;
use limu::scene::{
    Align, Item, Kind, MARKER_SIZE, Marker, MarkerType, Outline, Painter, Painting, Path, Placed,
    Point, Rect as SceneRect,
};
use limu_gpui::GpuiCanvas;

use super::FONT_SIZE;
use super::pixels::{Pixel, Pixels, Stroke, fill, floor_at, inward};
use crate::paint::{Fill, Pen, Rect, corner_inset};
use crate::rich_text::TextStyle;
use crate::theme::*;
use crate::{Font, Language};

/// Draws an item of a scene in the Platinum look, as a hook of
/// [`limu_gpui::paint_with`] given the window's point of the scene's origin
/// and the language its text was measured in: on whole pixels with a
/// [`Pen`], a node's box as a button and a group's as a group box, every
/// other outline, line and arrowhead a pixel at a time, and text in
/// [`Font::Small`]. Its picture, an icon or an image, is drawn as limu-gpui
/// draws one. The scene is drawn a pixel to its unit, as
/// [`layout_options`](super::layout_options) lays it out.
pub fn draw(
    p: &mut Painter<'_, GpuiCanvas<'_>>,
    it: &Item,
    s: &Resolved,
    origin: GpuiPoint<Px>,
    language: Language,
) {
    let origin = origin.map(|v| v.round());
    let d = Drawer { it, s };
    let mut pen = Pen::new(0, 0);
    pen.language = language;
    d.parts(&mut pen);
    d.ends(&mut pen);
    pen.paint(origin, p.canvas().window().0);
    p.picture(it, s);
    let mut pen = Pen::new(0, 0);
    pen.language = language;
    d.label(&mut pen);
    pen.paint(origin, p.canvas().window().0);
}

/// A color as the pen draws it, and how much of what is under it it covers,
/// out of 255.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Ink {
    pub color: u32,
    pub alpha: u8,
}

impl Ink {
    const NONE: Ink = Ink { color: 0, alpha: 0 };

    fn opaque(color: u32) -> Ink {
        Ink { color, alpha: 255 }
    }

    fn of(c: Color) -> Ink {
        Ink {
            color: u32::from(c.r) << 16 | u32::from(c.g) << 8 | u32::from(c.b),
            alpha: c.a,
        }
    }

    fn visible(self) -> bool {
        self.alpha > 0
    }

    pub fn rect(self, pen: &mut Pen, x: i32, y: i32, width: i32, height: i32) {
        if self.visible() {
            pen.translucent(x, y, width, height, self.color, self.alpha);
        }
    }

    /// The ink t of the way from this one to another, keeping this one's
    /// alpha.
    fn mix(self, other: u32, t: f64) -> Ink {
        let channel = |shift: u32| {
            let (u, v) = (
                f64::from((self.color >> shift) & 255),
                f64::from((other >> shift) & 255),
            );
            ((u + (v - u) * t).round() as u32) << shift
        };
        Ink {
            color: channel(16) | channel(8) | channel(0),
            alpha: self.alpha,
        }
    }
}

/// A box of the pen to draw in, from its top left corner.
struct Area<'p> {
    pen: &'p mut Pen,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Area<'_> {
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, ink: Ink) {
        ink.rect(self.pen, self.x + x, self.y + y, w, h);
    }
    fn hline(&mut self, x: i32, y: i32, w: i32, ink: Ink) {
        self.rect(x, y, w, 1, ink);
    }
    fn vline(&mut self, x: i32, y: i32, h: i32, ink: Ink) {
        self.rect(x, y, 1, h, ink);
    }
    fn dot(&mut self, x: i32, y: i32, ink: Ink) {
        self.rect(x, y, 1, 1, ink);
    }
    fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, ink: Ink) {
        self.hline(x, y, w, ink);
        self.hline(x, y + h - 1, w, ink);
        self.vline(x, y + 1, h - 2, ink);
        self.vline(x + w - 1, y + 1, h - 2, ink);
    }
    fn bevel(&mut self, x: i32, y: i32, w: i32, h: i32, top: Ink, bottom: Ink) {
        self.hline(x, y, w - 1, top);
        self.vline(x, y + 1, h - 2, top);
        self.hline(x + 1, y + h - 1, w - 1, bottom);
        self.vline(x + w - 1, y + 1, h - 2, bottom);
    }
    /// A frame with its corners cut to a radius, filled with a face, as
    /// [`Pen::round_frame`] draws one.
    fn round_frame(&mut self, w: i32, h: i32, radius: usize, line: Ink, face: Ink) {
        for row in 0..h {
            let inset = corner_inset(row, h, radius);
            if row == 0 || row == h - 1 {
                self.hline(inset, row, w - 2 * inset, line);
            } else {
                self.dot(inset, row, line);
                self.dot(w - inset - 1, row, line);
                self.hline(inset + 1, row, w - inset * 2 - 2, face);
            }
        }
        for row in 1..(radius as i32).min(h / 2) {
            for col in (corner_inset(row, h, radius) + 1)..corner_inset(row - 1, h, radius) {
                for (dx, dy) in [
                    (col, row),
                    (w - 1 - col, row),
                    (col, h - 1 - row),
                    (w - 1 - col, h - 1 - row),
                ] {
                    self.dot(dx, dy, line);
                }
            }
        }
    }
}

struct Drawer<'a> {
    it: &'a Item,
    s: &'a Resolved,
}

impl Drawer<'_> {
    /// How the item's lines go on pixels: an edge's and a casing's fall where
    /// they are, the lines of a shape toward its middle.
    fn stroke(&self) -> Stroke<Box<dyn Fn(Point) -> Pixel>> {
        let at: Box<dyn Fn(Point) -> Pixel> = if self.it.edge().is_none() && !self.casing() {
            Box::new(inward(self.it.bounds))
        } else {
            Box::new(floor_at)
        };
        Stroke {
            width: (self.s.stroke_width.round() as i32).max(1),
            dash: self
                .s
                .dash
                .iter()
                .map(|v| (v.round() as i32).max(1))
                .collect(),
            at,
        }
    }

    /// Whether the item is the casing under an edge that crosses a fill: the
    /// edge's line and its markers' outlines, stroked wider than the edge in
    /// the background.
    fn casing(&self) -> bool {
        self.it.role.as_str().ends_with(".casing")
    }

    fn parts(&self, pen: &mut Pen) {
        let (it, s) = (self.it, self.s);
        if self.casing() {
            // The edge's pixels and its markers' regions, all grown by the
            // casing's brush.
            let (mut px, st) = (Pixels::default(), self.stroke());
            for part in &it.parts {
                let lines = path_of(&part.outline).flatten(0.25);
                st.lines(&mut px, &lines);
                if part.paint != Painting::StrokeOnly {
                    px.extend(&fill(&lines));
                }
            }
            px.draw(pen, Ink::of(s.stroke.color));
            return;
        }
        for part in &it.parts {
            let (mut face, mut line) = (Ink::of(s.fill.color), Ink::of(s.stroke.color));
            match part.paint {
                Painting::FillAndStroke => {}
                Painting::StrokeOnly => face = Ink::NONE,
                Painting::FillOnly => line = Ink::NONE,
                Painting::Solid => face = Ink::of(s.line.color),
                Painting::Hollow => face = Ink::of(s.hollow.color),
                Painting::Shade => {
                    // The shade, or over a fill the author set, the shade at
                    // 60% over it.
                    (face, line) = (Ink::of(s.shade.color), Ink::NONE);
                    let f = Ink::of(s.fill.color);
                    if it.author.fill.is_some_and(|p| p.visible()) && f.visible() {
                        face = f.mix(face.color, 0.6);
                    }
                }
                Painting::Dot => (face, line) = (Ink::of(s.marker.color), Ink::NONE),
            }
            let path = match &part.outline {
                Outline::Box { rect, rounded } => {
                    self.draw_box(pen, *rect, *rounded, face, line);
                    continue;
                }
                Outline::Path(path) => path,
            };
            let lines = path.flatten(0.25);
            if !face.visible() {
                if line.visible() {
                    let mut px = Pixels::default();
                    self.stroke().lines(&mut px, &lines);
                    px.draw(pen, line);
                }
                continue;
            }
            let region = fill(&lines);
            if !line.visible() {
                region.draw(pen, face);
                continue;
            }
            // The frame, as wide as the stroke, inside the region, and the
            // face within it.
            let (mut frame, mut inner) = (Pixels::default(), region);
            for _ in 0..(s.stroke_width.round() as i32).max(1) {
                let b = inner.border();
                frame.extend(&b);
                inner = inner.without(&b);
            }
            inner.draw(pen, face);
            frame.draw(pen, line);
        }
    }

    /// Draws a box of an outline: a node's as a button, raised in its face; a
    /// group's as a group box, etched; any other filled and framed.
    fn draw_box(&self, pen: &mut Pen, b: SceneRect, rounded: bool, face: Ink, line: Ink) {
        let (x, y) = (b.x.round() as i32, b.y.round() as i32);
        let (w, h) = (
            (b.x + b.w).round() as i32 - x,
            (b.y + b.h).round() as i32 - y,
        );
        if w <= 0 || h <= 0 {
            return;
        }
        let radius = if rounded {
            (self.s.radius.round() as i32).clamp(0, 4) as usize
        } else {
            0
        };
        let mut area = Area { pen, x, y, w, h };
        match self.it.kind {
            Kind::Node if face.alpha == 255 && line.visible() && w >= 8 && h >= 8 => {
                button(&mut area, radius, face, line)
            }
            Kind::Group if line.visible() && self.it.author.stroke.is_none() => {
                area.rect(0, 0, w, h, face);
                group_box(&mut area);
            }
            _ => area.round_frame(w, h, radius, line, face),
        }
    }

    /// Draws an edge's markers, filled in the color of its line alone and
    /// stroked as it is: an arrowhead along a run across or down as the pixel
    /// arrowheads of old, growing a pixel each side every other row; any
    /// other as its parts fall on pixels, painted as [`Marker::outline`]
    /// says.
    fn ends(&self, pen: &mut Pen) {
        let Some(edge) = self.it.edge() else { return };
        let (c, line) = (Ink::of(self.s.marker.color), Ink::of(self.s.stroke.color));
        if !c.visible() && !line.visible() {
            return;
        }
        let (mut px, mut lined, mut under) =
            (Pixels::default(), Pixels::default(), Pixels::default());
        for m in edge.ends.iter().flatten() {
            if m.kind == MarkerType::ArrowPoint && (m.dx == 0.0) != (m.dy == 0.0) {
                arrowhead(&mut px, m);
                continue;
            }
            for part in m.outline() {
                let lines = path_of(&part.outline).flatten(0.25);
                match part.paint {
                    Painting::StrokeOnly => {
                        let stroke = Stroke {
                            width: 1,
                            dash: vec![],
                            at: floor_at,
                        };
                        stroke.lines(&mut lined, &lines);
                    }
                    Painting::Hollow => {
                        let region = fill(&lines);
                        lined.extend(&region.border());
                        under.extend(&region);
                    }
                    _ => px.extend(&fill(&lines)),
                }
            }
        }
        under.without(&px).draw(pen, Ink::of(self.s.hollow.color));
        lined.without(&px).draw(pen, line);
        px.draw(pen, c);
    }

    /// Draws the item's label in [`Font::Small`], each line centered on the
    /// label's point, or starting or ending there, its capitals in the middle
    /// of the line's height, a sub span lowered and a sup span raised, and an
    /// underlined span underlined; with a halo, drawn about it in the halo's
    /// color first. A turned label is turned about its point a pixel at a
    /// time, a quarter turn keeping its pixels whole.
    fn label(&self, pen: &mut Pen) {
        let Some(placed) = &self.it.label else { return };
        if placed.label.is_empty() {
            return;
        }
        let c = Ink::of(self.s.text.color);
        if !c.visible() {
            return;
        }
        let first = pen.fills.len();
        let halo = Ink::of(self.s.halo.color);
        if halo.visible() && self.s.halo_width > 0.0 {
            let r = ((self.s.halo_width / 2.0) as i32).max(1);
            for oy in -r..=r {
                for ox in -r..=r {
                    if (ox != 0 || oy != 0) && ox.abs() + oy.abs() <= r + r / 2 {
                        text(pen, placed, ox, oy, halo);
                    }
                }
            }
        }
        text(pen, placed, 0, 0, c);
        if placed.rotate != 0.0 {
            let fills = pen.fills.split_off(first);
            turn(pen, fills, placed.at, placed.rotate);
        }
    }
}

/// A part's outline as a path: its own, or its box's.
fn path_of(outline: &Outline) -> Path {
    match outline {
        Outline::Path(p) => p.clone(),
        Outline::Box { rect, .. } => {
            let mut p = Path::new();
            p.round_rect(*rect, 0.0);
            p
        }
    }
}

/// Draws a box as [`crate::draw_button`] draws one: framed, with two rings of
/// bevel inside, lit from the top left, in the shades of its face.
fn button(a: &mut Area, radius: usize, face: Ink, frame: Ink) {
    let (w, h) = (a.w, a.h);
    let (tl1, tl2) = (face.mix(WHITE, 1.0), face.mix(WHITE, 0.5));
    let (br1, br2) = (face.mix(BLACK, 5.0 / 13.0), face.mix(BLACK, 3.0 / 13.0));
    if radius == 0 {
        a.rect(0, 0, w, h, face);
        a.frame(0, 0, w, h, frame);
        a.bevel(1, 1, w - 2, h - 2, tl1, br1);
        a.bevel(2, 2, w - 4, h - 4, tl2, br2);
        return;
    }
    a.round_frame(w, h, 3, frame, face);
    a.hline(3, 1, w - 6, tl1);
    a.vline(1, 3, h - 6, tl1);
    a.dot(2, 2, tl1);
    a.hline(3, h - 2, w - 6, br1);
    a.vline(w - 2, 3, h - 6, br1);
    a.dot(w - 3, h - 3, br1);
    a.hline(3, 2, w - 7, tl2);
    a.vline(2, 3, h - 7, tl2);
    a.hline(4, h - 3, w - 7, br2);
    a.vline(w - 3, 4, h - 7, br2);
}

/// Draws the etched frame of [`crate::draw_group_box`] about a box.
fn group_box(a: &mut Area) {
    let (w, h) = (a.w, a.h);
    let (gray, white) = (Ink::opaque(GRAY_7), Ink::opaque(WHITE));
    a.frame(0, 0, w - 1, h - 1, gray);
    a.hline(1, 1, w - 3, white);
    a.vline(1, 2, h - 4, white);
    a.hline(0, h - 1, w, white);
    a.vline(w - 1, 0, h, white);
}

/// Puts an arrowhead of [`MARKER_SIZE`] rows on pixels, its tip on the pixel
/// before the marker's tip, in line with the edge's pixels.
pub(crate) fn arrowhead(px: &mut Pixels, m: &Marker) {
    let (dx, dy) = (m.dx.round() as i32, m.dy.round() as i32);
    // The tip's pixel: the one the line's last half pixel falls in.
    let mut tip = floor_at(Point::new(m.at.x - m.dx * 0.5, m.at.y - m.dy * 0.5));
    if dx == 0 {
        tip.x = m.at.x.floor() as i32;
    } else {
        tip.y = m.at.y.floor() as i32;
    }
    for row in 0..MARKER_SIZE as i32 {
        let half = (f64::from(row) * 0.42).round() as i32;
        for k in -half..=half {
            // Back along the edge by row, across it by k.
            px.add(tip.x - dx * row + dy * k, tip.y - dy * row + dx * k);
        }
    }
}

/// The style a face of limu's is drawn in.
pub(crate) fn text_style(f: Face) -> TextStyle {
    TextStyle {
        bold: f.contains(Face::BOLD),
        italic: f.contains(Face::ITALIC),
        underline: f.contains(Face::UNDERLINE),
        scale: 1,
    }
}

/// How far a span in a face is lowered: a sub span a fifth of
/// [`FONT_SIZE`], and a sup span raised a third of it, as limu's backends
/// shift them, at the faces' one size.
fn shift(f: Face) -> i32 {
    let size = FONT_SIZE as i32;
    if f.contains(Face::SUP) {
        -size / 3
    } else if f.contains(Face::SUB) {
        size / 5
    } else {
        0
    }
}

/// Draws the lines of a label, moved by ox, oy, in an ink.
fn text(pen: &mut Pen, placed: &Placed, ox: i32, oy: i32, ink: Ink) {
    let (f, l) = (Font::Small, &placed.label);
    // A span is measured without the tracking after its last glyph, so the
    // spans of a line are set that far apart, and the line placed by its
    // width as they are set.
    let tracking = f.measure("ii") - 2 * f.measure("i");
    let lh = l.line_height.round() as i32;
    let cap = f.cap_height();
    for (i, line) in l.lines.iter().enumerate() {
        let w: i32 = line.iter().map(|sp| sp.width.round() as i32).sum::<i32>()
            + tracking * (line.len() as i32 - 1).max(0);
        let x = match placed.align {
            Align::Start => placed.at.x,
            Align::End => placed.at.x - f64::from(w),
            Align::Middle => placed.at.x - f64::from(w) / 2.0,
        };
        let mut x = x.round() as i32 + ox;
        // Each line's top on the pixel it falls in, so that lines a fraction
        // of a pixel apart gather no error.
        let top = (placed.at.y - l.height / 2.0 + i as f64 * l.line_height).round() as i32 + oy;
        let baseline = top + (lh - cap) / 2 + cap;
        for sp in line {
            if !sp.text.is_empty() && sp.icon.is_none() {
                let first = pen.fills.len();
                pen.text_styled(
                    x,
                    baseline + shift(sp.face),
                    &sp.text,
                    f,
                    ink.color,
                    text_style(sp.face),
                );
                for fill in &mut pen.fills[first..] {
                    fill.alpha = ink.alpha;
                }
            }
            x += sp.width.round() as i32 + tracking;
        }
    }
}

/// Puts fills back on the pen turned by degrees, clockwise, about a point
/// on the grid: by a quarter turn rect by rect, and by any other angle pixel
/// by pixel, each pixel of the turned box taking the one it came from.
fn turn(pen: &mut Pen, fills: Vec<Fill>, about: Point, degrees: f64) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (cx, cy) = (about.x, about.y);
    let rotate = |x: f64, y: f64| {
        let (dx, dy) = (x - cx, y - cy);
        (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
    };
    if (degrees / 90.0).fract() == 0.0 {
        let (sin, cos) = (sin.round(), cos.round());
        let rotate = |x: f64, y: f64| {
            let (dx, dy) = (x - cx, y - cy);
            (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
        };
        for f in fills {
            let r = f.rect;
            let (x0, y0) = rotate(f64::from(r.x), f64::from(r.y));
            let (x1, y1) = rotate(f64::from(r.x + r.width), f64::from(r.y + r.height));
            pen.fills.push(Fill {
                rect: Rect {
                    x: x0.min(x1).round() as i32,
                    y: y0.min(y1).round() as i32,
                    width: (x1 - x0).abs().round() as i32,
                    height: (y1 - y0).abs().round() as i32,
                },
                ..f
            });
        }
        return;
    }
    // Each pixel's ink, the last drawn on it kept.
    let mut inks = std::collections::HashMap::new();
    let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
    let (mut x1, mut y1) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for f in &fills {
        let r = f.rect;
        for y in r.y..r.y + r.height {
            for x in r.x..r.x + r.width {
                inks.insert((x, y), (f.color, f.alpha));
            }
        }
        for (x, y) in [
            (r.x, r.y),
            (r.x + r.width, r.y),
            (r.x, r.y + r.height),
            (r.x + r.width, r.y + r.height),
        ] {
            let (tx, ty) = rotate(f64::from(x), f64::from(y));
            (x0, y0, x1, y1) = (x0.min(tx), y0.min(ty), x1.max(tx), y1.max(ty));
        }
    }
    if inks.is_empty() {
        return;
    }
    // The way back from a turned pixel's center to the one it came from.
    let back = |x: f64, y: f64| {
        let (dx, dy) = (x - cx, y - cy);
        (cx + dx * cos + dy * sin, cy - dx * sin + dy * cos)
    };
    for y in y0.floor() as i32..y1.ceil() as i32 {
        let mut run: Option<(i32, i32, (u32, u8))> = None;
        for x in x0.floor() as i32..=x1.ceil() as i32 {
            let (sx, sy) = back(f64::from(x) + 0.5, f64::from(y) + 0.5);
            let ink = inks.get(&(sx.floor() as i32, sy.floor() as i32)).copied();
            match (&mut run, ink) {
                (Some((_, width, run_ink)), Some(ink)) if *run_ink == ink => *width += 1,
                _ => {
                    if let Some((start, width, (color, alpha))) = run.take() {
                        pen.translucent(start, y, width, 1, color, alpha);
                    }
                    run = ink.map(|ink| (x, 1, ink));
                }
            }
        }
        if let Some((start, width, (color, alpha))) = run {
            pen.translucent(start, y, width, 1, color, alpha);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // An arrowhead down a column ends on the pixel above its tip, centered
    // on the column, and one up it on the pixel at its tip.
    #[test]
    fn arrowheads() {
        for (at, dx, dy) in [
            (Point::new(10., 20.), 0., 1.),
            (Point::new(10., 20.), 0., -1.),
            (Point::new(20., 10.), 1., 0.),
        ] {
            let m = Marker {
                kind: MarkerType::ArrowPoint,
                at,
                dx,
                dy,
            };
            let mut px = Pixels::default();
            arrowhead(&mut px, &m);
            let tip = (
                (at.x - f64::max(dx, 0.)) as i32,
                (at.y - f64::max(dy, 0.)) as i32,
            );
            assert_eq!(px.len(), 1 + 1 + 3 + 3 + 5 + 5 + 7 + 7, "{m:?}");
            assert!(px.has(tip.0, tip.1), "{m:?}: tip {tip:?}");
            let back = (tip.0 - dx as i32 * 7, tip.1 - dy as i32 * 7);
            let across = (dy as i32 * 3, dx as i32 * 3);
            assert!(
                px.has(back.0 + across.0, back.1 + across.1)
                    && px.has(back.0 - across.0, back.1 - across.1),
                "{m:?}: base not 7 wide about {back:?}"
            );
        }
    }

    // A label turned a quarter keeps its pixels whole, and one turned by
    // any other angle has as many pixels about as it had.
    #[test]
    fn turned_text() {
        let mut pen = Pen::new(0, 0);
        let fills = vec![Fill {
            rect: Rect {
                x: 10,
                y: 20,
                width: 6,
                height: 2,
            },
            color: BLACK,
            alpha: 255,
        }];
        turn(&mut pen, fills.clone(), Point::new(10., 20.), -90.);
        assert_eq!(
            pen.fills[0].rect,
            Rect {
                x: 10,
                y: 14,
                width: 2,
                height: 6
            }
        );
        let mut pen = Pen::new(0, 0);
        turn(&mut pen, fills, Point::new(10., 20.), -45.);
        let n: i32 = pen.fills.iter().map(|f| f.rect.width * f.rect.height).sum();
        assert!((9..=15).contains(&n), "{n} pixels");
    }
}
