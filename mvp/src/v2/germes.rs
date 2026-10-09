//! GERMES hexagon — computed geometry of Q³ (the ±1 cube).
//!
//! `docs/germes-mapping.md` states the hexagon is the projection of the cube
//! along its space diagonal (correct) and then *lists* antipode pairs by hand.
//! Two of the three listed pairs are not antipodes (they are 60° apart on the
//! hexagon). Here nothing is listed: positions, opposites and the two
//! triangles are derived from the cube, and the documented table is checked
//! against them in the tests.

use super::alphabet::Symbol;
use std::f64::consts::PI;

/// A vertex of Q³: each coordinate is −1 or +1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Q3 {
    pub x: i8,
    pub y: i8,
    pub z: i8,
}

impl Q3 {
    pub const fn new(x: i8, y: i8, z: i8) -> Self {
        Self { x, y, z }
    }

    /// Parse the `--+` notation used throughout the docs.
    pub fn parse(code: &str) -> Option<Q3> {
        let mut it = code.chars();
        let mut c = [0i8; 3];
        for slot in c.iter_mut() {
            *slot = match it.next()? {
                '+' => 1,
                '-' => -1,
                _ => return None,
            };
        }
        if it.next().is_some() {
            return None;
        }
        Some(Q3::new(c[0], c[1], c[2]))
    }

    pub fn code(&self) -> String {
        [self.x, self.y, self.z]
            .iter()
            .map(|v| if *v > 0 { '+' } else { '-' })
            .collect()
    }

    /// Number of `+` coordinates (Hamming weight from the `---` pole).
    pub fn weight(&self) -> u8 {
        [self.x, self.y, self.z].iter().filter(|v| **v > 0).count() as u8
    }

    pub fn antipode(&self) -> Q3 {
        Q3::new(-self.x, -self.y, -self.z)
    }

    /// `---` and `+++` project onto the centre of the hexagon.
    pub fn is_pole(&self) -> bool {
        matches!(self.weight(), 0 | 3)
    }

    /// Orthogonal projection onto the plane ⟂ (1,1,1), in the orthonormal
    /// basis e₁ = (1,−1,0)/√2, e₂ = (1,1,−2)/√6.
    pub fn project(&self) -> (f64, f64) {
        let (x, y, z) = (self.x as f64, self.y as f64, self.z as f64);
        let u = (x - y) / 2f64.sqrt();
        let v = (x + y - 2.0 * z) / 6f64.sqrt();
        (u, v)
    }

    /// Angle of the projected point in degrees, `[0, 360)`.
    pub fn angle_deg(&self) -> f64 {
        let (u, v) = self.project();
        let a = v.atan2(u) * 180.0 / PI;
        if a < 0.0 {
            a + 360.0
        } else {
            a
        }
    }
}

pub fn all_vertices() -> [Q3; 8] {
    let mut out = [Q3::new(-1, -1, -1); 8];
    let mut i = 0;
    for x in [-1, 1] {
        for y in [-1, 1] {
            for z in [-1, 1] {
                out[i] = Q3::new(x, y, z);
                i += 1;
            }
        }
    }
    out
}

/// The six non-pole vertices in true cyclic (counter-clockwise) order.
pub fn hexagon_cyclic() -> Vec<Q3> {
    let mut v: Vec<Q3> = all_vertices()
        .into_iter()
        .filter(|q| !q.is_pole())
        .collect();
    v.sort_by(|a, b| a.angle_deg().partial_cmp(&b.angle_deg()).unwrap());
    v
}

/// Opposite vertex on the hexagon. Geometrically this *is* the cube antipode.
pub fn hexagon_opposite(v: Q3) -> Q3 {
    v.antipode()
}

/// Weight-1 triangle ("awakening" in the docs).
pub fn triangle_awakening() -> Vec<Q3> {
    hexagon_cyclic()
        .into_iter()
        .filter(|q| q.weight() == 1)
        .collect()
}

/// Weight-2 triangle ("consolidation").
pub fn triangle_consolidation() -> Vec<Q3> {
    hexagon_cyclic()
        .into_iter()
        .filter(|q| q.weight() == 2)
        .collect()
}

/// A domain of the hexagon as assigned in `docs/specification.md` §3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Domain {
    pub name: &'static str,
    pub vertex: Q3,
    pub symbol: Symbol,
}

/// The v0.1 domain assignment, verbatim from the specification.
pub fn domains_v01() -> Vec<Domain> {
    vec![
        Domain {
            name: "Devices",
            vertex: Q3::parse("--+").unwrap(),
            symbol: Symbol::Directed,
        },
        Domain {
            name: "Payments",
            vertex: Q3::parse("-+-").unwrap(),
            symbol: Symbol::Diamond,
        },
        Domain {
            name: "Users",
            vertex: Q3::parse("+--").unwrap(),
            symbol: Symbol::Triangle,
        },
        Domain {
            name: "Logistics",
            vertex: Q3::parse("-++").unwrap(),
            symbol: Symbol::Unique,
        },
        Domain {
            name: "Content",
            vertex: Q3::parse("+-+").unwrap(),
            symbol: Symbol::Shared,
        },
        Domain {
            name: "Analytics",
            vertex: Q3::parse("++-").unwrap(),
            symbol: Symbol::Simple,
        },
    ]
}

/// Antipode pairs exactly as written in `docs/germes-mapping.md`.
pub fn documented_antipodes_v01() -> [(Q3, Q3); 3] {
    [
        (Q3::parse("--+").unwrap(), Q3::parse("-++").unwrap()),
        (Q3::parse("-+-").unwrap(), Q3::parse("+-+").unwrap()),
        (Q3::parse("+--").unwrap(), Q3::parse("++-").unwrap()),
    ]
}

/// Entries of the documented table that are *not* antipodes, each with the
/// correct partner. Empty means the document is right.
pub fn antipode_table_errors() -> Vec<((Q3, Q3), Q3)> {
    documented_antipodes_v01()
        .into_iter()
        .filter(|(a, b)| a.antipode() != *b)
        .map(|(a, b)| ((a, b), a.antipode()))
        .collect()
}

/// Domain pairs that sit opposite each other (natural homes for `✕`).
pub fn domain_antipodes() -> Vec<(Domain, Domain)> {
    let ds = domains_v01();
    let mut out = Vec::new();
    for (i, a) in ds.iter().enumerate() {
        for b in ds.iter().skip(i + 1) {
            if a.vertex.antipode() == b.vertex {
                out.push((a.clone(), b.clone()));
            }
        }
    }
    out
}

/// Render the hexagon as a small ASCII figure in true cyclic order.
pub fn render_ascii() -> String {
    let ds = domains_v01();
    let name_of = |q: Q3| {
        ds.iter()
            .find(|d| d.vertex == q)
            .map(|d| d.name)
            .unwrap_or("?")
    };
    let cyc = hexagon_cyclic();
    let mut s = String::new();
    s.push_str("GERMES hexagon (cube projected along --- ↔ +++), counter-clockwise:\n");
    for q in &cyc {
        s.push_str(&format!(
            "  {:>5}°  {}  w={}  {:<10} opposite: {} ({})\n",
            q.angle_deg().round() as i64,
            q.code(),
            q.weight(),
            name_of(*q),
            q.antipode().code(),
            name_of(q.antipode())
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poles_project_to_centre() {
        for p in ["---", "+++"] {
            let (u, v) = Q3::parse(p).unwrap().project();
            assert!(u.abs() < 1e-12 && v.abs() < 1e-12);
        }
    }

    #[test]
    fn six_vertices_form_a_regular_hexagon() {
        let cyc = hexagon_cyclic();
        assert_eq!(cyc.len(), 6);
        let angles: Vec<f64> = cyc.iter().map(|q| q.angle_deg()).collect();
        for w in angles.windows(2) {
            assert!(
                (w[1] - w[0] - 60.0).abs() < 1e-9,
                "not 60° apart: {angles:?}"
            );
        }
        // equal radius
        let r: Vec<f64> = cyc
            .iter()
            .map(|q| {
                let (u, v) = q.project();
                (u * u + v * v).sqrt()
            })
            .collect();
        for x in &r {
            assert!((x - r[0]).abs() < 1e-9);
        }
        // weights alternate 1,2,1,2,…
        let w: Vec<u8> = cyc.iter().map(|q| q.weight()).collect();
        assert_eq!(w, vec![1, 2, 1, 2, 1, 2]);
    }

    #[test]
    fn true_cyclic_order() {
        let codes: Vec<String> = hexagon_cyclic().iter().map(|q| q.code()).collect();
        assert_eq!(codes, vec!["+--", "++-", "-+-", "-++", "--+", "+-+"]);
    }

    #[test]
    fn documented_antipode_table_has_two_errors() {
        let errs = antipode_table_errors();
        assert_eq!(errs.len(), 2, "{errs:?}");
        let codes: Vec<(String, String, String)> = errs
            .iter()
            .map(|((a, b), c)| (a.code(), b.code(), c.code()))
            .collect();
        assert!(codes.contains(&("--+".into(), "-++".into(), "++-".into())));
        assert!(codes.contains(&("+--".into(), "++-".into(), "-++".into())));
    }

    #[test]
    fn opposite_is_180_degrees() {
        for q in hexagon_cyclic() {
            let d = (q.angle_deg() - hexagon_opposite(q).angle_deg()).abs();
            assert!((d - 180.0).abs() < 1e-9);
        }
    }

    #[test]
    fn triangles_are_the_star_of_david() {
        assert_eq!(triangle_awakening().len(), 3);
        assert_eq!(triangle_consolidation().len(), 3);
        // each triangle is equilateral: consecutive angles differ by 120°
        for tri in [triangle_awakening(), triangle_consolidation()] {
            let a: Vec<f64> = tri.iter().map(|q| q.angle_deg()).collect();
            assert!((a[1] - a[0] - 120.0).abs() < 1e-9 && (a[2] - a[1] - 120.0).abs() < 1e-9);
        }
    }

    #[test]
    fn domain_antipodes_follow_geometry() {
        let pairs = domain_antipodes();
        let names: Vec<(&str, &str)> = pairs.iter().map(|(a, b)| (a.name, b.name)).collect();
        assert_eq!(pairs.len(), 3);
        assert!(names.contains(&("Devices", "Analytics")));
        assert!(names.contains(&("Payments", "Content")));
        assert!(names.contains(&("Users", "Logistics")));
    }
}
