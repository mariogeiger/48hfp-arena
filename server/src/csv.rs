use crate::config::SiteConfig;
use versus_shared::Film;

/// Split one CSV line into fields, honoring double-quoted fields (which may
/// contain commas) and `""` escapes. Surrounding whitespace/quotes are trimmed
/// by the caller via `clean`.
fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
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
            ',' if !in_quotes => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
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
/// The first line is the header row; columns are matched by name.
pub fn parse_csv(content: &str, cfg: &SiteConfig) -> Vec<Film> {
    let mut lines = content.lines();
    let Some(header_line) = lines.next() else {
        return Vec::new();
    };
    let headers: Vec<String> = split_csv_line(header_line)
        .iter()
        .map(|s| clean(s))
        .collect();
    let col = |name: &str| headers.iter().position(|h| h == name);

    let label_idx = col(&cfg.columns.label);
    let sublabel_idx = cfg.columns.sublabel.as_deref().and_then(col);
    let detail_idx = cfg.columns.detail.as_deref().and_then(col);
    let image_idx = cfg.columns.image.as_deref().and_then(col);
    let link_idx = cfg.columns.link.as_deref().and_then(col);

    lines
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .filter_map(|(i, line)| {
            let fields = split_csv_line(line);
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
            let video_url = get(link_idx);

            Some(Film {
                id: i + 1,
                title,
                team,
                city,
                poster_url,
                video_url,
            })
        })
        .collect()
}
