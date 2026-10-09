//! Canonical Star Gate alphabet.
//!
//! v1 carries three alphabets that drifted apart: the Unicode table in
//! `README.md` / `docs/specification.md` (8 symbols), the Rust enum
//! `StarGateSymbol` (10 symbols: `Dot` and `Angle` added), and the keyboard
//! glyph keys in `docs/font-stargate-map.md` (9 keys, two of which — `$`
//! lever and `%` dense — are not relations at all).
//!
//! This module is the single source of truth: every symbol has exactly one
//! name, one Unicode form, at most one glyph key, a declared-saturation
//! *prior*, a direction flag and the edge kinds it may label. The M2M line
//! format sketched in `examples/m2m_glyph_line.md` (which had no parser) is
//! implemented here with a round-trip guarantee.

use std::fmt;

/// Connectivity degree (unchanged meaning: sparse / jamming / rigid).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Degree {
    D1,
    D2,
    D3,
}

impl fmt::Display for Degree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Relation symbol. Superset of the README table (adds `Dot`, `Angle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Symbol {
    Dot,
    Simple,
    Directed,
    Angle,
    Triangle,
    Diamond,
    Shared,
    Unique,
    Conflict,
    Hyper,
}

/// Role of an edge in the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Control,
    Data,
    Ownership,
    HyperMember,
    Diff,
}

/// One row of the canonical table.
#[derive(Clone, Copy, Debug)]
pub struct SymbolSpec {
    pub symbol: Symbol,
    pub name: &'static str,
    pub unicode: &'static str,
    /// Keyboard character in the Solan fonts. `None` = not printable there yet
    /// (a proposal for the font is in the audit, see `docs/audit/`).
    pub glyph_key: Option<char>,
    /// Declared saturation prior in `[0,1]`; see `relation::SaturationModel`.
    pub prior: f64,
    pub directed: bool,
    pub kinds: &'static [Kind],
}

use Kind::*;

pub const TABLE: [SymbolSpec; 10] = [
    SymbolSpec {
        symbol: Symbol::Dot,
        name: "dot",
        unicode: "·",
        glyph_key: Some('.'),
        prior: 0.00,
        directed: false,
        kinds: &[Control, Data],
    },
    SymbolSpec {
        symbol: Symbol::Simple,
        name: "simple",
        unicode: "—",
        glyph_key: Some('"'),
        prior: 0.00,
        directed: false,
        kinds: &[Control, Data],
    },
    SymbolSpec {
        symbol: Symbol::Directed,
        name: "directed",
        unicode: "→",
        glyph_key: Some('>'),
        prior: 0.10,
        directed: true,
        kinds: &[Control, Data],
    },
    SymbolSpec {
        symbol: Symbol::Angle,
        name: "angle",
        unicode: "∠",
        glyph_key: Some('('),
        prior: 0.30,
        directed: true,
        kinds: &[Control, Data],
    },
    SymbolSpec {
        symbol: Symbol::Triangle,
        name: "triangle",
        unicode: "△",
        glyph_key: Some('*'),
        prior: 0.35,
        directed: true,
        kinds: &[Data],
    },
    SymbolSpec {
        symbol: Symbol::Diamond,
        name: "diamond",
        unicode: "◇",
        glyph_key: Some('\''),
        prior: 0.40,
        directed: false,
        kinds: &[Data, Ownership],
    },
    SymbolSpec {
        symbol: Symbol::Shared,
        name: "shared",
        unicode: "□̸",
        glyph_key: Some('&'),
        prior: 0.40,
        directed: false,
        kinds: &[Ownership],
    },
    SymbolSpec {
        symbol: Symbol::Unique,
        name: "unique",
        unicode: "▲",
        glyph_key: Some('!'),
        prior: 0.50,
        directed: true,
        kinds: &[Ownership],
    },
    SymbolSpec {
        symbol: Symbol::Conflict,
        name: "conflict",
        unicode: "✕",
        glyph_key: Some(')'),
        prior: 0.85,
        directed: false,
        kinds: &[Ownership],
    },
    SymbolSpec {
        symbol: Symbol::Hyper,
        name: "hyper",
        unicode: "⬡",
        glyph_key: Some('#'),
        prior: 0.90,
        directed: false,
        kinds: &[HyperMember],
    },
];

impl Symbol {
    pub fn spec(self) -> &'static SymbolSpec {
        TABLE
            .iter()
            .find(|s| s.symbol == self)
            .expect("every Symbol has a TABLE row")
    }
    pub fn prior(self) -> f64 {
        self.spec().prior
    }
    pub fn unicode(self) -> &'static str {
        self.spec().unicode
    }
    pub fn glyph_key(self) -> Option<char> {
        self.spec().glyph_key
    }
    pub fn is_directed(self) -> bool {
        self.spec().directed
    }
    pub fn allows(self, kind: Kind) -> bool {
        self.spec().kinds.contains(&kind)
    }
    /// Degree implied by the prior alone (what the docs call "recommended degree").
    pub fn prior_degree(self) -> Degree {
        degree_for(self.prior(), 0.25, 0.75)
    }
    pub fn from_glyph_key(c: char) -> Option<Symbol> {
        TABLE
            .iter()
            .find(|s| s.glyph_key == Some(c))
            .map(|s| s.symbol)
    }
    pub fn from_unicode(s: &str) -> Option<Symbol> {
        TABLE.iter().find(|r| r.unicode == s).map(|r| r.symbol)
    }
    pub fn from_name(s: &str) -> Option<Symbol> {
        TABLE
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(s))
            .map(|r| r.symbol)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.unicode())
    }
}

/// Shared threshold function so every layer classifies identically.
pub fn degree_for(saturation: f64, d2_low: f64, d3_low: f64) -> Degree {
    if saturation < d2_low {
        Degree::D1
    } else if saturation < d3_low {
        Degree::D2
    } else {
        Degree::D3
    }
}

// ---------------------------------------------------------------------------
// M2M line format
// ---------------------------------------------------------------------------

/// Modifiers are glyph keys that are *not* relations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    /// `%` — force the edge into D3 regardless of computed saturation.
    Dense,
}

/// One parsed M2M statement.
#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    /// `A <glyph> B [%]`
    Relation {
        from: String,
        symbol: Symbol,
        to: String,
        modifiers: Vec<Modifier>,
    },
    /// `Group # m1 m2 m3 …`
    Hyper { label: String, members: Vec<String> },
    /// `Bal`, `Rei a b +1`, `RAG battery life`, `$` …
    Command { name: String, args: Vec<String> },
    /// Blank line or `//` comment.
    Empty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

fn is_glyph_token(tok: &str) -> Option<Symbol> {
    let mut chars = tok.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return Symbol::from_unicode(tok);
    }
    Symbol::from_glyph_key(c).or_else(|| Symbol::from_unicode(tok))
}

/// Parse one line. Grammar (whitespace separated tokens):
///
/// ```text
/// line     := empty | comment | relation | hyper | command
/// relation := IDENT GLYPH IDENT ('%')*
/// hyper    := IDENT '#' IDENT IDENT*
/// command  := IDENT arg*                 (first token is not followed by a glyph)
/// GLYPH    := one of the glyph keys in TABLE, or the unicode symbol
/// ```
pub fn parse_line(line_no: usize, raw: &str) -> Result<Statement, ParseError> {
    let line = raw.trim();
    if line.is_empty() || line.starts_with("//") {
        return Ok(Statement::Empty);
    }
    let toks: Vec<&str> = line.split_whitespace().collect();
    // `Lnk a g b` is the keyboard macro form of a relation.
    let toks: Vec<&str> = if toks
        .first()
        .map(|t| t.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false)
    {
        toks[1..].to_vec()
    } else {
        toks
    };
    if toks.len() >= 3 {
        if let Some(sym) = is_glyph_token(toks[1]) {
            if sym == Symbol::Hyper {
                return Ok(Statement::Hyper {
                    label: toks[0].to_string(),
                    members: toks[2..].iter().map(|s| s.to_string()).collect(),
                });
            }
            let mut modifiers = Vec::new();
            for extra in &toks[3..] {
                match *extra {
                    "%" => modifiers.push(Modifier::Dense),
                    other => {
                        return Err(ParseError {
                            line: line_no,
                            message: format!("unexpected token `{other}` after relation"),
                        })
                    }
                }
            }
            return Ok(Statement::Relation {
                from: toks[0].to_string(),
                symbol: sym,
                to: toks[2].to_string(),
                modifiers,
            });
        }
    }
    if toks.len() == 2 && is_glyph_token(toks[1]).is_some() {
        return Err(ParseError {
            line: line_no,
            message: "relation is missing its target node".into(),
        });
    }
    Ok(Statement::Command {
        name: toks[0].to_string(),
        args: toks[1..].iter().map(|s| s.to_string()).collect(),
    })
}

pub fn parse(text: &str) -> Result<Vec<Statement>, ParseError> {
    text.lines()
        .enumerate()
        .map(|(i, l)| parse_line(i + 1, l))
        .filter(|r| !matches!(r, Ok(Statement::Empty)))
        .collect()
}

impl Statement {
    /// Serialise back to one M2M line using glyph keys (font-printable).
    pub fn to_m2m(&self) -> String {
        match self {
            Statement::Relation {
                from,
                symbol,
                to,
                modifiers,
            } => {
                let g = symbol
                    .glyph_key()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| symbol.unicode().to_string());
                let mut s = format!("{from} {g} {to}");
                for m in modifiers {
                    match m {
                        Modifier::Dense => s.push_str(" %"),
                    }
                }
                s
            }
            Statement::Hyper { label, members } => format!("{label} # {}", members.join(" ")),
            Statement::Command { name, args } => {
                if args.is_empty() {
                    name.clone()
                } else {
                    format!("{name} {}", args.join(" "))
                }
            }
            Statement::Empty => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_consistent() {
        // every symbol once, every glyph key unique
        let mut keys = std::collections::BTreeSet::new();
        for row in TABLE.iter() {
            assert_eq!(row.symbol.spec().name, row.name);
            if let Some(k) = row.glyph_key {
                assert!(keys.insert(k), "duplicate glyph key {k:?}");
            }
            assert!((0.0..=1.0).contains(&row.prior));
        }
        assert_eq!(keys.len(), 10);
    }

    #[test]
    fn priors_match_documented_recommended_degrees() {
        // docs/connectivity-degree.md: · — → D1 ; ∠ △ ◇ □̸ → D2 ; ▲ → D2–D3 ; ✕ ⬡ → D3
        assert_eq!(Symbol::Dot.prior_degree(), Degree::D1);
        assert_eq!(Symbol::Simple.prior_degree(), Degree::D1);
        assert_eq!(Symbol::Directed.prior_degree(), Degree::D1);
        assert_eq!(Symbol::Angle.prior_degree(), Degree::D2);
        assert_eq!(Symbol::Triangle.prior_degree(), Degree::D2);
        assert_eq!(Symbol::Diamond.prior_degree(), Degree::D2);
        assert_eq!(Symbol::Shared.prior_degree(), Degree::D2);
        assert_eq!(Symbol::Unique.prior_degree(), Degree::D2);
        assert_eq!(Symbol::Conflict.prior_degree(), Degree::D3);
        assert_eq!(Symbol::Hyper.prior_degree(), Degree::D3);
    }

    #[test]
    fn parses_examples_from_m2m_glyph_line_md() {
        let text = "Transfer \" CheckBalance\nTransfer ) Refund\nRefund ' Balance\nPaymentsModule # Transfer Refund CheckBalance\n";
        let stmts = parse(text).unwrap();
        assert_eq!(stmts.len(), 4);
        assert_eq!(
            stmts[1],
            Statement::Relation {
                from: "Transfer".into(),
                symbol: Symbol::Conflict,
                to: "Refund".into(),
                modifiers: vec![]
            }
        );
        assert_eq!(
            stmts[3],
            Statement::Hyper {
                label: "PaymentsModule".into(),
                members: vec!["Transfer".into(), "Refund".into(), "CheckBalance".into()]
            }
        );
    }

    #[test]
    fn round_trip_and_unicode_and_macros() {
        let text = "A ✕ B\nLnk A ' B\nA \" B %\nBal\nRei A B +1\n// comment\n\n";
        let stmts = parse(text).unwrap();
        assert_eq!(stmts.len(), 5);
        assert!(matches!(
            &stmts[0],
            Statement::Relation {
                symbol: Symbol::Conflict,
                ..
            }
        ));
        assert!(matches!(
            &stmts[1],
            Statement::Relation {
                symbol: Symbol::Diamond,
                ..
            }
        ));
        assert!(
            matches!(&stmts[2], Statement::Relation { modifiers, .. } if modifiers == &vec![Modifier::Dense])
        );
        assert!(matches!(&stmts[3], Statement::Command { name, .. } if name == "Bal"));
        let again: Vec<Statement> = stmts
            .iter()
            .map(|s| parse_line(1, &s.to_m2m()).unwrap())
            .collect();
        assert_eq!(again, stmts);
    }

    #[test]
    fn reports_errors_with_line_numbers() {
        let err = parse("A ) B junk").unwrap_err();
        assert_eq!(err.line, 1);
        let err = parse("ok\nA )").unwrap_err();
        assert_eq!(err.line, 2);
    }
}
