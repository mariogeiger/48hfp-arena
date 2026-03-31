#!/usr/bin/env python3
"""Detect potential ballot stuffing in db.json.

A shill voter selects all films, then votes their film against every other film.
The signature: one film concentrates a large share of the user's total wins.
"""

import json
import sys
from collections import Counter
from pathlib import Path

DB_PATH = Path(__file__).parent / "db.json"
BANNED_PATH = Path(__file__).parent / "banned.txt"
DATA_CSV = Path(__file__).parent / "data.csv"

# A user is flagged if their top film has >= this fraction of their total wins
WIN_SHARE_THRESHOLD = 0.6
# Minimum votes to bother analyzing
MIN_VOTES = 20
# Minimum films voted on -- small counts are just strong preferences, not stuffing
MIN_FILMS = 40


def load_films():
    films = {}
    for i, line in enumerate(DATA_CSV.read_text().splitlines()[1:], 1):
        if line.startswith('"'):
            title = line.split('"')[1]
        else:
            title = line.split(",")[0].strip()
        films[i] = title
    return films


def load_banned():
    if not BANNED_PATH.exists():
        return set()
    return {
        line.strip()
        for line in BANNED_PATH.read_text().splitlines()
        if line.strip() and not line.strip().startswith("#")
    }


def analyze_user(state):
    outcomes = state.get("vote_outcomes", {})
    if len(outcomes) < MIN_VOTES:
        return None

    wins = Counter()
    for winner in outcomes.values():
        wins[winner] += 1

    total_wins = sum(wins.values())
    if total_wins == 0:
        return None

    top_film, top_wins = wins.most_common(1)[0]
    win_share = top_wins / total_wins

    voted_films = set()
    for a, b in state.get("compared_pairs", []):
        voted_films.add(a)
        voted_films.add(b)

    if win_share < WIN_SHARE_THRESHOLD or len(voted_films) < MIN_FILMS:
        return None

    return {
        "top_film": top_film,
        "top_wins": top_wins,
        "total_wins": total_wins,
        "win_share": win_share,
        "total_votes": len(state.get("compared_pairs", [])),
        "films_voted_on": len(voted_films),
    }


def main():
    data = json.loads(DB_PATH.read_text())
    films = load_films()
    banned = load_banned()

    flagged = []
    for uid, state in data["users"].items():
        result = analyze_user(state)
        if result:
            result["uid"] = uid
            result["banned"] = uid in banned
            flagged.append(result)

    flagged.sort(key=lambda r: r["win_share"], reverse=True)

    if not flagged:
        print("No suspicious voters found.")
        return

    print(f"{'Top Film':>40} {'Share':>7} {'Top W':>5} {'Votes':>6} {'Films':>5} {'Status':>8}  UID")
    print("-" * 110)
    for r in flagged:
        name = films.get(r["top_film"], f"#{r['top_film']}")
        status = "BANNED" if r["banned"] else "<<<"
        print(
            f"{name:>40} {r['win_share']:>6.0%} {r['top_wins']:>5} {r['total_votes']:>6} "
            f"{r['films_voted_on']:>5} {status:>8}  {r['uid']}"
        )


if __name__ == "__main__":
    main()
