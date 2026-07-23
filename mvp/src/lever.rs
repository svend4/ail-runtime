//! Step 6: LeverBalance — inverse pressures of clique vs independent set

use ail_mvp::connectivity::ConnectivityDegree;

/// Clique pressure (C) vs independence pressure (I).
///
/// After normalisation C + I = 1. The working mass of D2 edges is reported
/// separately as `d2_share`.
#[derive(Clone, Debug)]
pub struct LeverBalance {
    pub clique_pressure: f64, // C
    pub indep_pressure: f64,  // I
    pub d2_share: f64,        // fraction of edges that are D2
    pub product: f64,         // C * I
    pub tilted: bool,         // true if outside soft health window
}

impl LeverBalance {
    pub fn from_degree_counts(d1: usize, d2: usize, d3: usize) -> Self {
        let total = (d1 + d2 + d3).max(1) as f64;
        let c_raw = d3 as f64 / total;
        let i_raw = d1 as f64 / total;
        let d2_share = d2 as f64 / total;

        // Normalise C and I onto the residual mass (exclude pure D2)
        let s = (c_raw + i_raw).max(1e-9);
        let c = c_raw / s;
        let i = i_raw / s;
        let product = c * i;

        let tilted = (c - i).abs() >= 0.25 || product <= 0.15;

        Self {
            clique_pressure: c,
            indep_pressure: i,
            d2_share,
            product,
            tilted,
        }
    }

    pub fn from_degrees<'a, I>(degrees: I) -> Self
    where
        I: IntoIterator<Item = &'a ConnectivityDegree>,
    {
        let mut d1 = 0;
        let mut d2 = 0;
        let mut d3 = 0;
        for d in degrees {
            match d {
                ConnectivityDegree::D1 => d1 += 1,
                ConnectivityDegree::D2 => d2 += 1,
                ConnectivityDegree::D3 => d3 += 1,
            }
        }
        Self::from_degree_counts(d1, d2, d3)
    }

    pub fn print(&self) {
        println!("=== Lever Balance ===");
        println!("  C (clique / D3 side)     : {:.3}", self.clique_pressure);
        println!("  I (independent / D1 side): {:.3}", self.indep_pressure);
        println!("  D2 share                 : {:.3}", self.d2_share);
        println!("  product C*I              : {:.3}", self.product);
        println!(
            "  status                   : {}",
            if self.tilted {
                "TILTED — consider reinforce/decay"
            } else {
                "balanced"
            }
        );
        println!();
    }
}
