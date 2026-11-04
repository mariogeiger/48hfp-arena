use crate::config::SiteConfig;
use versus_shared::{Film, Video};

/// Split CSV text into records of fields, honoring double-quoted fields
/// (which may hold commas and line breaks) and `""` escapes. Surrounding
/// whitespace/quotes are trimmed by the caller via `clean`.
fn records(content: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => record.push(std::mem::take(&mut cur)),
            '\r' if !in_quotes => {}
            '\n' if !in_quotes => {
                record.push(std::mem::take(&mut cur));
                records.push(std::mem::take(&mut record));
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() || !record.is_empty() {
        record.push(cur);
        records.push(record);
    }
    records
}

fn clean(s: &str) -> String {
    s.trim().trim_matches('"').trim().to_string()
}

/// Replace `{Header}` tokens in a fallback template with this row's values.
fn apply_template(tpl: &str, headers: &[String], fields: &[String]) -> String {
    let mut out = tpl.to_string();
    for (h, f) in headers.iter().zip(fields.iter()) {
        out = out.replace(&format!("{{{}}}", h), &clean(f));
    }
    out
}

/// Parse `data.csv` into items using the column mapping from the site config.
/// The first record is the header row; columns are matched by name. An item's
/// id is its position among the non-blank records.
pub fn parse_csv(content: &str, cfg: &SiteConfig) -> Vec<Film> {
    let mut records = records(content).into_iter();
    let Some(header) = records.next() else {
        return Vec::new();
    };
    let headers: Vec<String> = header.iter().map(|s| clean(s)).collect();
    let col = |name: &str| headers.iter().position(|h| h == name);

    let label_idx = col(&cfg.columns.label);
    let sublabel_idx = cfg.columns.sublabel.as_deref().and_then(col);
    let detail_idx = cfg.columns.detail.as_deref().and_then(col);
    let image_idx = cfg.columns.image.as_deref().and_then(col);
    let link_idx = cfg.columns.link.as_deref().and_then(col);
    let description_idx = cfg.columns.description.as_deref().and_then(col);

    records
        .filter(|fields| !(fields.len() == 1 && fields[0].trim().is_empty()))
        .enumerate()
        .filter_map(|(i, fields)| {
            let get = |idx: Option<usize>| {
                idx.and_then(|j| fields.get(j))
                    .map(|s| clean(s))
                    .unwrap_or_default()
            };

            let title = get(label_idx);
            if title.is_empty() {
                return None;
            }
            let team = get(sublabel_idx);
            let city = get(detail_idx);

            let mut poster_url = get(image_idx);
            if poster_url.is_empty()
                && let Some(tpl) = &cfg.image_fallback
            {
                poster_url = apply_template(tpl, &headers, &fields);
            }
            let videos = Video::parse_lines(&get(link_idx));
            let synopsis = get(description_idx);

            Some(Film {
                id: i + 1,
                title,
                team,
                city,
                poster_url,
                videos,
                synopsis,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_fields_span_lines_and_blank_lines_take_no_id() {
        let cfg: SiteConfig = toml::from_str(
            "display_name = \"x\"\n[columns]\nlabel = \"Title\"\ndescription = \"Synopsis\"\n",
        )
        .unwrap();
        let csv = "Title,Synopsis\r\nA,\"One, two.\nThree \"\"four\"\".\"\r\n\r\nB,\n";
        let films = parse_csv(csv, &cfg);
        assert_eq!(films.len(), 2);
        assert_eq!(films[0].synopsis, "One, two.\nThree \"four\".");
        assert_eq!((films[1].id, films[1].title.as_str()), (2, "B"));
    }

    #[test]
    fn a_video_cell_holds_one_version_per_line() {
        let cfg: SiteConfig = toml::from_str(
            "display_name = \"x\"\n[columns]\nlabel = \"Title\"\nlink = \"Video\"\n",
        )
        .unwrap();
        let csv = "Title,Video\nA,https://a.ch/1\nB,\"Original https://b.ch/1\nDirector's cut https://b.ch/2\"\nC,\n";
        let films = parse_csv(csv, &cfg);
        assert_eq!(films[0].videos, Video::parse_lines("https://a.ch/1"));
        let labels: Vec<&str> = films[1].videos.iter().map(|v| v.label.as_str()).collect();
        assert_eq!(labels, ["Original", "Director's cut"]);
        assert!(films[2].videos.is_empty());
    }
}
