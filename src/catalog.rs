//! The Kessel index — the six objects in Issue 14.
//!
//! Static for now: this is the shape the Postgres-backed catalogue will
//! return, so screens can be written against it before the schema exists.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    pub k: &'static str,
    pub v: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Product {
    pub id: &'static str,
    pub num: &'static str,
    pub name: &'static str,
    pub cat: &'static str,
    /// Whole dollars — no fractional currency in the index.
    pub price: u32,
    pub stock: &'static str,
    /// The one-line entry used in the index listing.
    pub line: &'static str,
    pub blurb: &'static str,
    /// The buying desk's note, shown on the product page.
    pub note: &'static str,
    pub specs: &'static [Spec],
}

impl Product {
    pub fn price_label(&self) -> String {
        money(self.price)
    }

    pub fn in_stock(&self) -> bool {
        self.stock == "In stock"
    }

    fn haystack(&self) -> String {
        format!("{} {} {}", self.name, self.cat, self.blurb).to_lowercase()
    }
}

pub static PRODUCTS: &[Product] = &[
    Product {
        id: "loop",
        num: "01",
        name: "Loop One earbuds",
        cat: "Audio",
        price: 180,
        stock: "In stock",
        line: "Two mics, one dial, nine hours.",
        blurb: "Small enough to forget you're wearing them, and the case charges off the same cable as your laptop. The dial is the whole interface — turn for volume, press to let the room back in.",
        note: "The pair that survived a month of commuting in a coat pocket without a scratch on the case.",
        specs: &[
            Spec { k: "Battery", v: "9 h + 27 h case" },
            Spec { k: "Driver", v: "11 mm dynamic" },
            Spec { k: "Weight", v: "4.6 g each" },
            Spec { k: "Water", v: "IPX5" },
        ],
    },
    Product {
        id: "tone",
        num: "02",
        name: "Tonearm T1 turntable",
        cat: "Audio",
        price: 640,
        stock: "Two left",
        line: "Set the counterweight once. It holds.",
        blurb: "A belt-drive deck with a machined aluminium platter and no plastic anywhere you can see. Set the counterweight once and it holds.",
        note: "It is the least fussy turntable we have had on the desk, and the only one nobody argued about.",
        specs: &[
            Spec { k: "Drive", v: "Belt, 33 / 45" },
            Spec { k: "Cartridge", v: "MM, replaceable" },
            Spec { k: "Platter", v: "1.6 kg aluminium" },
            Spec { k: "Output", v: "Line / phono" },
        ],
    },
    Product {
        id: "mono",
        num: "03",
        name: "Mono Speaker 04",
        cat: "Audio",
        price: 420,
        stock: "In stock",
        line: "One grille, one knob, eleven hours.",
        blurb: "One speaker, one grille, one knob. Runs on mains or eleven hours of battery, and pairs to a second unit if you decide you want stereo after all.",
        note: "We left one in the studio kitchen for a year. It is still the thing people reach for.",
        specs: &[
            Spec { k: "Power", v: "40 W" },
            Spec { k: "Battery", v: "11 h" },
            Spec { k: "Inputs", v: "BT 5.3, 3.5 mm" },
            Spec { k: "Body", v: "Steel + ash" },
        ],
    },
    Product {
        id: "cam",
        num: "04",
        name: "Field Camera M6",
        cat: "Cameras",
        price: 1290,
        stock: "Pre-order",
        line: "Dials you can work in gloves.",
        blurb: "Full-frame, fully mechanical dials, and a shutter you can feel through your fingertips. Weather-sealed for the kind of trip where you stop checking the forecast.",
        note: "Hand it to someone who has never used a camera and they will still get the exposure right.",
        specs: &[
            Spec { k: "Sensor", v: "36 MP full-frame" },
            Spec { k: "Mount", v: "M6 / adapters" },
            Spec { k: "Shutter", v: "1/8000 s" },
            Spec { k: "Sealing", v: "IP53" },
        ],
    },
    Product {
        id: "watch",
        num: "05",
        name: "Kessel Watch 02",
        cat: "Wearables",
        price: 260,
        stock: "In stock",
        line: "Eighteen days between charges.",
        blurb: "Reads the time first and everything else second. Eighteen days between charges because the screen is not trying to be a phone.",
        note: "The only smartwatch on the desk that nobody took off at the end of the test.",
        specs: &[
            Spec { k: "Battery", v: "18 days" },
            Spec { k: "Display", v: "1.3\" memory LCD" },
            Spec { k: "Case", v: "40 mm steel" },
            Spec { k: "Water", v: "10 ATM" },
        ],
    },
    Product {
        id: "dac",
        num: "06",
        name: "Cassette DAC",
        cat: "Audio",
        price: 310,
        stock: "In stock",
        line: "Two jacks, so listening is shared.",
        blurb: "A palm-sized converter with a real volume wheel and two headphone jacks, so listening with someone else is not an accessory purchase.",
        note: "Plugs into a decade-old laptop and makes it sound like it cost more than it did.",
        specs: &[
            Spec { k: "DAC", v: "32-bit / 384 kHz" },
            Spec { k: "Outputs", v: "2 × 3.5 mm" },
            Spec { k: "Power", v: "USB-C, bus" },
            Spec { k: "Weight", v: "96 g" },
        ],
    },
];

/// Search chips: three categories, then price and availability.
pub const CATS: &[&str] = &["Audio", "Cameras", "Wearables", "Under $400", "In stock"];

pub fn find(id: &str) -> Option<&'static Product> {
    PRODUCTS.iter().find(|p| p.id == id)
}

/// `1290` → `"$1,290"`.
pub fn money(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + 3);
    out.push('$');
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// One filter chip's predicate. Category chips match the category; the last
/// two read a different field each.
fn chip_matches(chip: &str, p: &Product) -> bool {
    match chip {
        "Under $400" => p.price < 400,
        "In stock" => p.in_stock(),
        cat => p.cat == cat,
    }
}

/// Free-text over name/category/blurb, then every active chip (AND).
pub fn search<'a>(query: &str, filters: &'a [&'static str]) -> Vec<&'static Product> {
    let q = query.trim().to_lowercase();
    PRODUCTS
        .iter()
        .filter(|p| q.is_empty() || p.haystack().contains(&q))
        .filter(|p| filters.iter().all(|f| chip_matches(f, p)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_groups_thousands() {
        assert_eq!(money(180), "$180");
        assert_eq!(money(1290), "$1,290");
        assert_eq!(money(0), "$0");
    }

    #[test]
    fn filters_and_query_compose() {
        assert_eq!(search("", &[]).len(), PRODUCTS.len());
        assert_eq!(search("turntable", &[]).len(), 1);
        // Audio under $400, in stock: Loop One and the Cassette DAC.
        let ids: Vec<_> = search("", &["Audio", "Under $400", "In stock"])
            .iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, vec!["loop", "dac"]);
        assert!(search("turntable", &["Cameras"]).is_empty());
    }
}
