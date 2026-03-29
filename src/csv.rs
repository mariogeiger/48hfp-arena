use crate::models::Film;

pub fn parse_csv(content: &str) -> Vec<Film> {
    content.lines()
        .skip(1)
        .enumerate()
        .filter_map(|(i, line)| {
            let line = line.trim();
            let parts: Vec<&str> = line.splitn(3, ',').collect();
            let title = parts.first()?.trim().trim_matches('"').to_string();
            let team = parts.get(1)?.trim().to_string();
            let city = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
            let poster_url = format!(
                "https://www.48hourfilm.com/storage/posters/48HFP {} 2025 - {} - Poster - file 1.jpg",
                city, team
            );
            Some(Film { id: i + 1, title, team, city, poster_url })
        })
        .collect()
}
