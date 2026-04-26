use chrono::{DateTime, Utc};
use regex::Regex;
use scraper::{Html, Selector};

/// Base URL for 3GPP spec archive.
pub const BASE_URL: &str = "https://www.3gpp.org/ftp/Specs/archive/";

/// Struct holding parsed spec number parts.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SpecNumber {
    pub series: String,
    pub number: String,
}

impl std::fmt::Display for SpecNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.series, self.number)
    }
}

impl std::str::FromStr for SpecNumber {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_spec_number(s)
    }
}

/// Parse a `spec_number` into its two-character series and the trailing number part.
/// Accepts strings that start with two digits, optionally a dot, then at least one alphanumeric
/// and optionally a part and subpart(s).
/// Returns `Ok(SpecNumber)` on success, or `Err(String)` with a human-friendly message on failure.
pub fn parse_spec_number(spec: &str) -> Result<SpecNumber, String> {
    let re = Regex::new(r"^\d{2}\.?[A-Za-z0-9]+(-\d+)*$")
        .map_err(|e| format!("internal regex error: {}", e))?;
    if !re.is_match(spec) {
        return Err(format!(
            "invalid spec_number '{}': must start with two digits, optionally a dot, then at least one alphanumeric character and optionally a hyphen followed by one or more digits",
            spec
        ));
    }

    let series = &spec[0..2];
    let mut rest = &spec[2..];
    if rest.starts_with('.') {
        rest = &rest[1..];
    }

    Ok(SpecNumber {
        series: series.to_string(),
        number: rest.to_string(),
    })
}

/// Month of year with explicit numeric values 1..=12.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Month {
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

/// Simple filter holding a year and month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateFilter {
    pub year: u32,
    pub month: Month,
}

/// Version with nonnegative integer components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub editorial: u32,
}

impl std::cmp::PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => match self.minor.cmp(&other.minor) {
                std::cmp::Ordering::Equal => self.editorial.cmp(&other.editorial),
                ord => ord,
            },
            ord => ord,
        }
    }
}

/// Single spec item including version, date and URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecItem {
    pub version: Version,
    pub date: DateTime<Utc>,
    pub url: String,
}

impl std::fmt::Display for SpecItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:2}.{:2}.{:2} @ {} ({})",
            self.version.major,
            self.version.minor,
            self.version.editorial,
            self.date.to_rfc3339(),
            self.url
        )
    }
}

impl std::convert::TryFrom<u8> for Month {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Month::January),
            2 => Ok(Month::February),
            3 => Ok(Month::March),
            4 => Ok(Month::April),
            5 => Ok(Month::May),
            6 => Ok(Month::June),
            7 => Ok(Month::July),
            8 => Ok(Month::August),
            9 => Ok(Month::September),
            10 => Ok(Month::October),
            11 => Ok(Month::November),
            12 => Ok(Month::December),
            _ => Err(format!("invalid month value: {} (expected 1..=12)", value)),
        }
    }
}

impl std::str::FromStr for DateFilter {
    type Err = String;

    /// Parse a date string in YYYY-MM format into `DateFilter`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re =
            Regex::new(r"^(\d{4})-(\d{2})$").map_err(|e| format!("internal regex error: {}", e))?;
        let caps = re
            .captures(s)
            .ok_or_else(|| format!("invalid date '{}': must be YYYY-MM", s))?;
        let year: u32 = caps
            .get(1)
            .ok_or("missing year")?
            .as_str()
            .parse()
            .map_err(|e| format!("invalid year: {}", e))?;
        let month_num: u8 = caps
            .get(2)
            .ok_or("missing month")?
            .as_str()
            .parse()
            .map_err(|e| format!("invalid month: {}", e))?;
        let month = Month::try_from(month_num)?;
        Ok(DateFilter { year, month })
    }
}

fn parse_version(filename: &str) -> Option<Version> {
    let stem = std::path::Path::new(filename).file_stem()?.to_str()?;
    let parts: Vec<&str> = stem.split('-').collect();
    let ver_str = parts.last()?.trim_end_matches(".zip");
    match ver_str.len() {
        3 => {
            let chars: Vec<char> = ver_str.chars().collect();
            let to_digit = |c: char| -> Option<u32> {
                match c {
                    '0'..='9' => Some(c as u32 - '0' as u32),
                    'a'..='z' => Some(c as u32 - 'a' as u32 + 10),
                    'A'..='Z' => Some(c as u32 - 'A' as u32 + 10),
                    _ => None,
                }
            };
            Some(Version {
                major: to_digit(chars[0])?,
                minor: to_digit(chars[1])?,
                editorial: to_digit(chars[2])?,
            })
        }
        6 => Some(Version {
            major: ver_str.get(0..2)?.parse().ok()?,
            minor: ver_str.get(2..4)?.parse().ok()?,
            editorial: ver_str.get(4..6)?.parse().ok()?,
        }),
        _ => None,
    }
}

/// List specs matching provided filters.
///
/// This is a simple placeholder implementation that returns an empty list.
pub fn list(
    spec_number: SpecNumber,
    release: Option<u32>,
    date_filter: Option<DateFilter>,
) -> Result<Vec<SpecItem>, String> {
    let base =
        reqwest::Url::parse(BASE_URL).map_err(|e| format!("failed to parse BASE_URL: {}", e))?;
    let path = format!("{}_series/{}", spec_number.series, spec_number);
    let url = base
        .join(&path)
        .map_err(|e| format!("failed to join path to BASE_URL: {}", e))?;

    if !url.as_str().starts_with(BASE_URL) {
        return Err(format!(
            "security check failed: URL '{}' does not start with BASE_URL",
            url
        ));
    }

    let response = reqwest::blocking::get(url.clone())
        .map_err(|e| format!("failed to fetch URL '{}': {}", url, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "failed to fetch URL '{}': status code {}",
            url,
            response.status()
        ));
    }

    let body = response
        .text()
        .map_err(|e| format!("failed to read response body: {}", e))?;

    let document = Html::parse_document(&body);
    let (name_idx, date_idx) = find_header_indexes(&document)?;

    let row_selector =
        Selector::parse("tbody > tr").map_err(|e| format!("selector error: {:?}", e))?;
    let cell_selector = Selector::parse("td").map_err(|e| format!("selector error: {:?}", e))?;
    let link_selector =
        Selector::parse("a[href]").map_err(|e| format!("selector error: {:?}", e))?;

    let mut specs = Vec::new();

    for row in document.select(&row_selector) {
        let cells: Vec<_> = row.select(&cell_selector).collect();
        if cells.len() <= std::cmp::max(name_idx, date_idx) {
            continue;
        }

        let name_cell = cells[name_idx];
        let date_cell = cells[date_idx];

        let anchor = match name_cell.select(&link_selector).next() {
            Some(a) => a,
            None => continue,
        };

        let url = anchor.value().attr("href").unwrap_or("").to_string();
        let filename = anchor.text().collect::<String>();

        let date_str = date_cell.text().collect::<String>();
        let date_str = date_str.trim();

        let date = match chrono::NaiveDateTime::parse_from_str(date_str, "%Y/%m/%d %-H:%M") {
            Ok(dt) => DateTime::from_naive_utc_and_offset(dt, Utc),
            Err(_) => continue,
        };

        let version = match parse_version(&filename) {
            Some(v) => v,
            None => continue,
        };

        if let Some(rel) = release {
            if version.major != rel {
                continue;
            }
        }

        // If a `date_filter` is provided, derive `filter_start` and `filter_end`.
        // `filter_start` is the first day of that year/month at 00:00 UTC.
        // `filter_end` is three months after `filter_start` (exclusive upper bound).
        let (filter_start, filter_end): (Option<DateTime<Utc>>, Option<DateTime<Utc>>) =
            if let Some(df) = date_filter {
                // `from_ymd_opt` and `and_hms_opt` return Option to avoid panics on invalid dates.
                let start =
                    chrono::NaiveDate::from_ymd_opt(df.year as i32, df.month as u8 as u32, 1)
                        .and_then(|d| d.and_hms_opt(0, 0, 0))
                        .map(|naive_dt| DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc));
                let end = start
                    .clone()
                    .and_then(|s| s.checked_add_months(chrono::Months::new(3)));
                (start, end)
            } else {
                (None, None)
            };

        // If a `date_filter` was provided, exclude rows outside [filter_start, filter_end).
        if let Some(start) = filter_start {
            if date < start {
                continue;
            }
            if let Some(end) = filter_end {
                if date >= end {
                    continue;
                }
            }
        }

        specs.push(SpecItem { version, date, url });
    }

    // Sort by `version` in descending order: compare `major`, then `minor`, then `editorial`.
    specs.sort_by(|a, b| b.version.cmp(&a.version));

    Ok(specs)
}

/// Find the column indexes for "name" and "date" in the table header.
/// Returns `Ok((name_index, date_index))` on success.
pub fn find_header_indexes(document: &Html) -> Result<(usize, usize), String> {
    let selector = Selector::parse("thead > tr > th")
        .map_err(|e| format!("internal selector error: {:?}", e))?;

    let mut name_idx = None;
    let mut date_idx = None;

    for (i, element) in document.select(&selector).enumerate() {
        let text = element.text().collect::<String>().to_lowercase();
        if name_idx.is_none() && text.contains("name") {
            name_idx = Some(i);
        }
        if date_idx.is_none() && text.contains("date") {
            date_idx = Some(i);
        }
    }

    match (name_idx, date_idx) {
        (Some(n), Some(d)) => Ok((n, d)),
        _ => Err("failed to find 'name' and 'date' columns".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{SpecNumber, find_header_indexes, parse_spec_number};
    use scraper::Html;

    #[test]
    fn valid_examples() {
        assert_eq!(
            parse_spec_number("23a"),
            Ok(SpecNumber {
                series: "23".to_string(),
                number: "a".to_string()
            })
        );
        assert_eq!(
            parse_spec_number("23.a"),
            Ok(SpecNumber {
                series: "23".to_string(),
                number: "a".to_string()
            })
        );
        assert_eq!(
            parse_spec_number("00Z"),
            Ok(SpecNumber {
                series: "00".to_string(),
                number: "Z".to_string()
            })
        );
        assert_eq!(
            parse_spec_number("99.1"),
            Ok(SpecNumber {
                series: "99".to_string(),
                number: "1".to_string()
            })
        );
        assert_eq!(
            parse_spec_number("45B6"),
            Ok(SpecNumber {
                series: "45".to_string(),
                number: "B6".to_string()
            })
        );
    }

    #[test]
    fn invalid_examples() {
        assert!(parse_spec_number("2a").is_err()); // only one leading digit
        assert!(parse_spec_number(".23a").is_err()); // starts with dot
        assert!(parse_spec_number("ab23").is_err()); // doesn't start with digits
        assert!(parse_spec_number("23.").is_err()); // dot with no alnum after
        assert!(parse_spec_number("").is_err());
    }

    #[test]
    fn test_find_header_indexes() {
        let html = r#"
            <table style="margin-left:20px">
                <thead>
                  <tr>
				  <th style="text-align:center">
					<br>
					<input style="" title="Select all" type="checkbox" onclick="selectAll(this.checked);"> 
					</th>
                    <th>&nbsp;</th>
                    <th><a href="?sortby=name">sort by name</a>/<a href="?sortby=namerev">desc</a>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;</th>
                    <th><a href="?sortby=date">sort by date</a>/<a href="?sortby=daterev">desc</a>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;</th> 
                    <th><a href="?sortby=size">sort by size</a>/<a href="?sortby=sizerev">desc</a></th>
                  </tr>
                </thead>
        "#;
        let doc = Html::parse_document(html);
        assert_eq!(find_header_indexes(&doc), Ok((2, 3)));
    }
}
