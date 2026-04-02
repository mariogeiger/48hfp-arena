use filmrank_shared::Film;

pub fn parse_csv(content: &str) -> Vec<Film> {
    content
        .lines()
        .skip(1)
        .enumerate()
        .filter_map(|(i, line)| {
            let line = line.trim();
            let parts: Vec<&str> = line.splitn(5, ',').collect();
            let title = parts.first()?.trim().trim_matches('"').to_string();
            let team = parts.get(1)?.trim().to_string();
            let city = parts
                .get(2)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            let poster_override = parts
                .get(3)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let poster_url = poster_override.unwrap_or_else(|| {
                format!(
                    "https://www.48hourfilm.com/storage/posters/48HFP {} 2025 - {} - Poster - file 1.jpg",
                    city, team
                )
            });
            let video_url = parts
                .get(4)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_default();
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
