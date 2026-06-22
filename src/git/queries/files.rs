use git2::Repository;
use std::{collections::HashSet, path::Path};

const SCORE_EXACT_PATH: i64 = 1_000_000;
const SCORE_EXACT_BASENAME: i64 = 900_000;
const SCORE_BASENAME_PREFIX: i64 = 800_000;
const SCORE_SEGMENT_PREFIX: i64 = 700_000;
const SCORE_SUBSTRING: i64 = 600_000;
const SCORE_FUZZY: i64 = 400_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSearchResult {
    pub path: String,
    pub score: i64,
    pub matched_indices: Vec<usize>,
}

struct TermMatch {
    score: i64,
    matched_indices: Vec<usize>,
}

pub fn search_tracked_files(repo: &Repository, query: &str, limit: usize) -> Result<Vec<FileSearchResult>, git2::Error> {
    let Some(workdir) = repo.workdir() else {
        return Ok(Vec::new());
    };

    if query.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let gix_repo = gix::open(workdir).map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let paths = tracked_file_paths_gix(&gix_repo)?;

    Ok(rank_file_paths(&paths, query, limit))
}

fn tracked_file_paths_gix(repo: &gix::Repository) -> Result<Vec<String>, git2::Error> {
    let Some(workdir) = repo.workdir() else {
        return Ok(Vec::new());
    };

    let index = repo.index().map_err(|error| git2::Error::from_str(&error.to_string()))?;
    let mut seen = HashSet::new();
    let mut paths = Vec::new();

    for entry in index.entries() {
        let Ok(path) = std::str::from_utf8(entry.path(&index)) else {
            continue;
        };

        let path = normalize_path(path);
        if path.is_empty() || is_git_internal_path(&path) || !seen.insert(path.clone()) {
            continue;
        }

        if workdir.join(Path::new(&path)).is_file() {
            paths.push(path);
        }
    }

    Ok(paths)
}

pub fn rank_file_paths(paths: &[String], query: &str, limit: usize) -> Vec<FileSearchResult> {
    let terms = normalize_query(query);
    if terms.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut seen = HashSet::new();
    let mut results: Vec<FileSearchResult> = paths
        .iter()
        .filter_map(|path| {
            let path = normalize_path(path);
            if path.is_empty() || is_git_internal_path(&path) || !seen.insert(path.clone()) {
                return None;
            }

            score_path(&path, &terms).map(|(score, matched_indices)| FileSearchResult { path, score, matched_indices })
        })
        .collect();

    results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.chars().count().cmp(&b.path.chars().count())).then_with(|| a.path.cmp(&b.path)));
    results.truncate(limit);
    results
}

fn normalize_query(query: &str) -> Vec<String> {
    normalize_path(query).split_whitespace().map(normalize_path).filter(|term| !term.is_empty()).map(|term| term.to_ascii_lowercase()).collect()
}

fn normalize_path(path: &str) -> String {
    let normalized = path.trim().replace('\\', "/");
    strip_leading_dot_slashes(&normalized).to_string()
}

fn strip_leading_dot_slashes(mut path: &str) -> &str {
    while let Some(stripped) = path.strip_prefix("./") {
        path = stripped;
    }
    path
}

fn is_git_internal_path(path: &str) -> bool {
    path == ".git" || path.starts_with(".git/")
}

fn score_path(path: &str, terms: &[String]) -> Option<(i64, Vec<usize>)> {
    let lower_path = path.to_ascii_lowercase();
    let mut score = 0;
    let mut matched_indices = Vec::new();

    for term in terms {
        let term_match = match_term(path, &lower_path, term)?;
        score += term_match.score;
        matched_indices.extend(term_match.matched_indices);
    }

    matched_indices.sort_unstable();
    matched_indices.dedup();
    Some((score, matched_indices))
}

fn match_term(path: &str, lower_path: &str, term: &str) -> Option<TermMatch> {
    if term.is_empty() {
        return None;
    }

    let basename_start = basename_start_byte(lower_path);
    let basename = &lower_path[basename_start..];
    let mut best = None;

    if lower_path == term {
        consider_match(&mut best, contiguous_match(path, lower_path, term, 0, basename_start, SCORE_EXACT_PATH));
    }

    if basename == term {
        consider_match(&mut best, contiguous_match(path, lower_path, term, basename_start, basename_start, SCORE_EXACT_BASENAME));
    }

    if basename.starts_with(term) {
        consider_match(&mut best, contiguous_match(path, lower_path, term, basename_start, basename_start, SCORE_BASENAME_PREFIX));
    }

    for start in segment_start_bytes(lower_path) {
        if lower_path[start..].starts_with(term) {
            consider_match(&mut best, contiguous_match(path, lower_path, term, start, basename_start, SCORE_SEGMENT_PREFIX));
        }
    }

    for start in occurrence_starts(lower_path, term) {
        consider_match(&mut best, contiguous_match(path, lower_path, term, start, basename_start, SCORE_SUBSTRING));
    }

    if let Some(term_match) = fuzzy_match(lower_path, term, basename_start) {
        consider_match(&mut best, term_match);
    }

    best
}

fn consider_match(best: &mut Option<TermMatch>, candidate: TermMatch) {
    if best.as_ref().is_none_or(|current| candidate.score > current.score) {
        *best = Some(candidate);
    }
}

fn contiguous_match(path: &str, lower_path: &str, term: &str, start_byte: usize, basename_start: usize, base_score: i64) -> TermMatch {
    let start = byte_to_char_index(path, start_byte);
    let term_len = term.chars().count();
    let path_len = path.chars().count() as i64;
    let mut score = base_score + term_len as i64 * 150 - path_len * 2 - start as i64 * 25;

    if start_byte == 0 {
        score += 6_000;
    }
    if is_segment_start_byte(lower_path, start_byte) {
        score += 3_000;
    } else if is_boundary_byte(lower_path, start_byte) {
        score += 1_500;
    }
    if start_byte >= basename_start {
        score += 2_500;
    }
    if start_byte == basename_start {
        score += 3_000;
    }

    TermMatch { score, matched_indices: (start..start + term_len).collect() }
}

fn fuzzy_match(lower_path: &str, term: &str, basename_start: usize) -> Option<TermMatch> {
    let path_chars: Vec<char> = lower_path.chars().collect();
    let term_chars: Vec<char> = term.chars().collect();
    if term_chars.is_empty() {
        return None;
    }

    let char_bytes: Vec<usize> = lower_path.char_indices().map(|(idx, _)| idx).collect();
    let mut best = None;

    for start in path_chars.iter().enumerate().filter_map(|(idx, ch)| (*ch == term_chars[0]).then_some(idx)) {
        let mut matched = vec![start];
        let mut search_from = start + 1;
        let mut found_all = true;

        for term_char in term_chars.iter().skip(1) {
            if let Some(next) = path_chars.iter().enumerate().skip(search_from).find_map(|(idx, ch)| (*ch == *term_char).then_some(idx)) {
                matched.push(next);
                search_from = next + 1;
            } else {
                found_all = false;
                break;
            }
        }

        if !found_all {
            continue;
        }

        let first = *matched.first().unwrap();
        let last = *matched.last().unwrap();
        let gaps: usize = matched.windows(2).map(|pair| pair[1].saturating_sub(pair[0] + 1)).sum();
        let consecutive = matched.windows(2).filter(|pair| pair[1] == pair[0] + 1).count();
        let start_byte = char_bytes[first];
        let path_len = path_chars.len() as i64;
        let span = last.saturating_sub(first) + 1;

        let mut score = SCORE_FUZZY + term_chars.len() as i64 * 120 + consecutive as i64 * 1_000 - gaps as i64 * 80 - span as i64 * 30 - first as i64 * 25 - path_len * 2;
        if start_byte == 0 {
            score += 3_000;
        }
        if is_segment_start_byte(lower_path, start_byte) {
            score += 2_000;
        } else if is_boundary_byte(lower_path, start_byte) {
            score += 1_000;
        }
        if start_byte >= basename_start {
            score += 1_500;
        }
        if start_byte == basename_start {
            score += 1_500;
        }

        consider_match(&mut best, TermMatch { score, matched_indices: matched });
    }

    best
}

fn basename_start_byte(path: &str) -> usize {
    path.rfind('/').map(|idx| idx + 1).unwrap_or(0)
}

fn byte_to_char_index(path: &str, byte_index: usize) -> usize {
    path[..byte_index].chars().count()
}

fn segment_start_bytes(path: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(path.char_indices().filter_map(|(idx, ch)| (ch == '/').then_some(idx + 1)).filter(|idx| *idx < path.len()));
    starts
}

fn is_segment_start_byte(path: &str, byte_index: usize) -> bool {
    byte_index == 0 || path[..byte_index].chars().next_back().is_some_and(|ch| ch == '/')
}

fn is_boundary_byte(path: &str, byte_index: usize) -> bool {
    byte_index == 0 || path[..byte_index].chars().next_back().is_some_and(|ch| matches!(ch, '/' | '_' | '-' | '.' | ' '))
}

fn occurrence_starts(path: &str, needle: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut search_from = 0;

    while search_from <= path.len() {
        let Some(relative_start) = path[search_from..].find(needle) else {
            break;
        };
        let start = search_from + relative_start;
        starts.push(start);

        let next = path[start..].chars().next().map(|ch| start + ch.len_utf8()).unwrap_or(path.len());
        if next <= search_from {
            break;
        }
        search_from = next;
    }

    starts
}

#[cfg(test)]
#[path = "../../tests/git/queries/files.rs"]
mod tests;
