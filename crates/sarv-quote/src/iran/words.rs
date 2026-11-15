//! Persian number-to-words and currency amount-in-words formatting.

const ONES: [&str; 10] = ["", "یک", "دو", "سه", "چهار", "پنج", "شش", "هفت", "هشت", "نه"];
const TEENS: [&str; 10] = [
    "ده",
    "یازده",
    "دوازده",
    "سیزده",
    "چهارده",
    "پانزده",
    "شانزده",
    "هفده",
    "هجده",
    "نوزده",
];
const TENS: [&str; 10] = [
    "",
    "",
    "بیست",
    "سی",
    "چهل",
    "پنجاه",
    "شصت",
    "هفتاد",
    "هشتاد",
    "نود",
];
const HUNDREDS: [&str; 10] = [
    "",
    "صد",
    "دویست",
    "سیصد",
    "چهارصد",
    "پانصد",
    "ششصد",
    "هفتصد",
    "هشتصد",
    "نهصد",
];
const SCALES: [&str; 6] = ["", "هزار", "میلیون", "میلیارد", "تریلیون", "کوادریلیون"];

const MAX_WORDABLE: u128 = 1_000_000_000_000_000_000;

fn three_digits(n: u16) -> String {
    let h = (n / 100) as usize;
    let rem = n % 100;
    let t = (rem / 10) as usize;
    let o = (rem % 10) as usize;

    let mut parts: Vec<&str> = Vec::new();
    if h > 0 {
        parts.push(HUNDREDS[h]);
    }
    if (10..20).contains(&rem) {
        parts.push(TEENS[(rem - 10) as usize]);
    } else {
        if t > 0 {
            parts.push(TENS[t]);
        }
        if o > 0 {
            parts.push(ONES[o]);
        }
    }
    parts.join(" و ")
}

/// 1_234_567 → "یک میلیون و دویست و سی و چهار هزار و پانصد و شصت و هفت". Zero → "صفر".
pub fn persian_words(n: u128) -> String {
    if n == 0 {
        return "صفر".to_string();
    }
    if n >= MAX_WORDABLE {
        return n.to_string();
    }

    let mut groups: Vec<(usize, u16)> = Vec::new();
    let mut rem = n;
    let mut scale_idx = 0;
    while rem > 0 {
        let g = (rem % 1000) as u16;
        if g > 0 {
            groups.push((scale_idx, g));
        }
        rem /= 1000;
        scale_idx += 1;
    }
    groups.reverse();

    let mut parts: Vec<String> = Vec::with_capacity(groups.len());
    for (pos, &(scale, g)) in groups.iter().enumerate() {
        let is_leading = pos == 0;
        if scale == 1 && g == 1 && is_leading {
            parts.push("هزار".to_string());
        } else {
            let w = three_digits(g);
            if scale == 0 {
                parts.push(w);
            } else {
                parts.push(format!("{w} {}", SCALES[scale]));
            }
        }
    }
    parts.join(" و ")
}

/// Amount in words with its unit: rials → "… ریال"; toman uses whole tomans and, when
/// `rials % 10 != 0`, appends "و N ریال".
pub fn amount_in_words(rials: i128, toman: bool) -> String {
    let prefix = if rials < 0 { "منفی " } else { "" };
    let abs = rials.unsigned_abs();
    if !toman {
        return format!("{prefix}{} ریال", persian_words(abs));
    }
    let tomans = abs / 10;
    let rem = abs % 10;
    if rem == 0 {
        format!("{prefix}{} تومان", persian_words(tomans))
    } else if tomans == 0 {
        format!("{prefix}{} ریال", persian_words(rem))
    } else {
        format!("{prefix}{} تومان و {} ریال", persian_words(tomans), persian_words(rem))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persian_words_table() {
        let cases: &[(u128, &str)] = &[
            (0, "صفر"),
            (1, "یک"),
            (10, "ده"),
            (11, "یازده"),
            (19, "نوزده"),
            (20, "بیست"),
            (21, "بیست و یک"),
            (100, "صد"),
            (101, "صد و یک"),
            (110, "صد و ده"),
            (999, "نهصد و نود و نه"),
            (1000, "هزار"),
            (1001, "هزار و یک"),
            (2000, "دو هزار"),
            (10_000, "ده هزار"),
            (100_000, "صد هزار"),
            (1_000_000, "یک میلیون"),
            (1_001_000, "یک میلیون و یک هزار"),
            (1_234_567, "یک میلیون و دویست و سی و چهار هزار و پانصد و شصت و هفت"),
            (1_000_000_000, "یک میلیارد"),
            (12_345_678_901, "دوازده میلیارد و سیصد و چهل و پنج میلیون و ششصد و هفتاد و هشت هزار و نهصد و یک"),
            (1_000_000_000_000, "یک تریلیون"),
            (1_000_000_000_000_000, "یک کوادریلیون"),
            (1_000_000_000_000_000_000, "1000000000000000000"),
        ];
        for &(n, expected) in cases {
            assert_eq!(persian_words(n), expected, "for n={n}");
        }
    }

    #[test]
    fn amount_in_words_units_and_signs() {
        let cases: &[(i128, bool, &str)] = &[
            (0, false, "صفر ریال"),
            (0, true, "صفر تومان"),
            (1_000, false, "هزار ریال"),
            (1_000, true, "صد تومان"),
            (125, true, "دوازده تومان و پنج ریال"),
            (5, true, "پنج ریال"),
            (-50_000, false, "منفی پنجاه هزار ریال"),
            (-50_005, true, "منفی پنج هزار تومان و پنج ریال"),
        ];
        for &(rials, toman, expected) in cases {
            assert_eq!(amount_in_words(rials, toman), expected, "for rials={rials}, toman={toman}");
        }
    }
}
