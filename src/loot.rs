use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

static RE_STANDALONE_MONEY: OnceLock<Regex> = OnceLock::new();
static RE_CURRENCY: OnceLock<Regex> = OnceLock::new();
// Store loot cache entries with Arc to avoid cloning large vectors/strings repeatedly.
pub type LootCacheEntry = (Arc<[String]>, Arc<String>);
static LOOT_CACHE: OnceLock<Mutex<HashMap<String, LootCacheEntry>>> = OnceLock::new();

pub fn standalone_money_regex() -> &'static Regex {
    RE_STANDALONE_MONEY.get_or_init(|| Regex::new(r"^\d+\s*(gp|sp|cp)$").unwrap())
}
pub fn currency_regex() -> &'static Regex {
    RE_CURRENCY.get_or_init(|| Regex::new(r"^(\d+)\s*(cp|sp|gp)$").unwrap())
}
fn loot_cache() -> &'static Mutex<HashMap<String, LootCacheEntry>> {
    LOOT_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn parse_loot_into_items(desc: &str) -> Vec<String> {
    // Replace " and " with commas to unify splitting, then split on commas
    let unified = desc.replace(" and ", ",");
    let money_re = standalone_money_regex();
    let mut out = Vec::new();
    for raw in unified.split(',') {
        let mut s = raw.trim();
        if s.is_empty() {
            continue;
        }
        for prefix in [
            "A ", "a ", "An ", "an ", "some ", "Some ", "several ", "Several ",
        ] {
            if let Some(rest) = s.strip_prefix(prefix) {
                s = rest;
            }
        }
        s = s.trim();
        if s.is_empty() {
            continue;
        }
        let ends_with_currency = s.ends_with(" gp") || s.ends_with(" sp") || s.ends_with(" cp");
        if money_re.is_match(s) || s.contains('(') || !ends_with_currency {
            out.push(capitalize_first_letter(s));
        }
    }
    if out.is_empty() {
        out.push(desc.to_string());
    }
    out
}

pub fn capitalize_first_letter(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + it.as_str(),
    }
}

fn add_article(item: &str) -> String {
    let first = item.chars().next().unwrap_or('a');
    let article = matches!(
        first,
        'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U'
    )
    .then_some("an")
    .unwrap_or("a");
    format!("{} {}", article, item)
}

pub fn format_items_for_display(items: &[String]) -> String {
    let cre = currency_regex();
    let formatted: Vec<String> = items
        .iter()
        .map(|item| {
            if let Some(caps) = cre.captures(item)
                && let (Some(a), Some(cur)) = (caps.get(1), caps.get(2))
            {
                let amount = a.as_str();
                let expanded = match cur.as_str() {
                    "cp" => {
                        if amount == "1" {
                            "copper piece"
                        } else {
                            "copper pieces"
                        }
                    }
                    "sp" => {
                        if amount == "1" {
                            "silver piece"
                        } else {
                            "silver pieces"
                        }
                    }
                    "gp" => {
                        if amount == "1" {
                            "gold piece"
                        } else {
                            "gold pieces"
                        }
                    }
                    _ => return add_article(item),
                };
                return format!("{} {}", amount, expanded);
            }
            add_article(item)
        })
        .collect();
    match formatted.len() {
        0 => String::new(),
        1 => formatted[0].clone(),
        2 => format!("{} and {}", formatted[0], formatted[1]),
        _ => {
            let (last, rest) = formatted.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

pub fn parse_and_format_loot_cached(desc: &str) -> LootCacheEntry {
    if let Ok(cache) = loot_cache().lock()
        && let Some(entry) = cache.get(desc)
    {
        return entry.clone();
    }
    let items_vec = parse_loot_into_items(desc);
    let formatted_str = format_items_for_display(&items_vec);
    let entry: LootCacheEntry = (
        Arc::from(items_vec.into_boxed_slice()),
        Arc::new(formatted_str),
    );
    if let Ok(mut cache) = loot_cache().lock() {
        cache.insert(desc.to_string(), entry.clone());
    }
    entry
}
