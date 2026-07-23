//! Transformation algebra (`topology-deck.html:181-193`): `s = v·e`, `r`,
//! `T`, `Φ`. Pure, RNG-free, fully portable — no `mulberry32` needed here.
//! Task 5.

/// One row of the deck's 13-industry table.
#[derive(Debug, Clone, PartialEq)]
pub struct Industry {
    pub name: String,
    pub lane: u8,
    pub c: f64,
    pub v: f64,
    pub e: f64,
}

/// The transformation-algebra outputs for a set of industries.
#[derive(Debug, Clone, PartialEq)]
pub struct Economy {
    pub r: f64,
    pub big_s: f64,
    pub cv: f64,
    pub t: Vec<f64>,
    pub phi: f64,
    pub rent_imperial: f64,
    pub rent_internal: f64,
}

/// `s_i = v_i·e_i`; `S = Σs_i`; `CV = Σ(c_i+v_i)`; `r = S/CV`;
/// `T_i = r·(c_i+v_i) − s_i`; `rent_imperial = −Σ_{lane=0}T_i`;
/// `rent_internal = −Σ_{lane=1}T_i`; `Φ = Σ_{T_i>0}T_i`.
/// Iterates `inds` in slice order (deterministic).
pub fn transform(inds: &[Industry]) -> Economy {
    let s: Vec<f64> = inds.iter().map(|i| i.v * i.e).collect();
    let big_s: f64 = s.iter().sum();
    let cv: f64 = inds.iter().map(|i| i.c + i.v).sum();
    let r = big_s / cv;

    let t: Vec<f64> = inds
        .iter()
        .zip(s.iter())
        .map(|(i, s_i)| r * (i.c + i.v) - s_i)
        .collect();

    let mut rent_imperial = 0.0;
    let mut rent_internal = 0.0;
    let mut phi = 0.0;
    for (i, t_i) in inds.iter().zip(t.iter()) {
        if i.lane == 0 {
            rent_imperial -= t_i;
        }
        if i.lane == 1 {
            rent_internal -= t_i;
        }
        if *t_i > 0.0 {
            phi += t_i;
        }
    }

    Economy {
        r,
        big_s,
        cv,
        t,
        phi,
        rent_imperial,
        rent_internal,
    }
}

/// The deck's 13-industry fixture table (`topology-deck.html`, scout-verified
/// c/v/e/lane; Task 5).
pub fn deck_industries() -> Vec<Industry> {
    let rows: [(&str, u8, f64, f64, f64); 13] = [
        ("export agriculture", 0, 14.0, 10.0, 3.2),
        ("textile assembly", 0, 10.0, 9.0, 3.0),
        ("extractive mining", 0, 26.0, 7.0, 2.6),
        ("migrant agriculture", 1, 9.0, 8.0, 2.4),
        ("meatpacking", 1, 12.0, 8.0, 2.2),
        ("logistics depots", 1, 18.0, 9.0, 1.9),
        ("carceral labor", 1, 4.0, 5.0, 4.0),
        ("defense aerospace", 2, 34.0, 8.0, 1.1),
        ("suburban construction", 2, 20.0, 9.0, 1.2),
        ("professional strata", 2, 16.0, 11.0, 0.9),
        ("pharma IP", 3, 44.0, 6.0, 1.0),
        ("platform rents", 3, 52.0, 7.0, 1.1),
        ("finance", 3, 60.0, 5.0, 0.8),
    ];
    rows.into_iter()
        .map(|(name, lane, c, v, e)| Industry {
            name: name.to_string(),
            lane,
            c,
            v,
            e,
        })
        .collect()
}
