use crate::engine::Hit;
use crate::models::support_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 60 * MINUTE_MS;
const DAY_MS: i64 = 24 * HOUR_MS;
const WEEK_MS: i64 = 7 * DAY_MS;
const MAX_BUCKETS: i64 = 2000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoggedHit {
    pub t: f64,
    pub phrase: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PhraseCount {
    pub phrase: String,
    pub count: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatsBucket {
    pub start_ms: i64,
    pub count: u32,
    pub segments: Vec<PhraseCount>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatsResult {
    pub total: u32,
    pub buckets: Vec<StatsBucket>,
    pub breakdown: Vec<PhraseCount>,
    pub phrases: Vec<String>,
    pub too_many: Option<u32>,
}

pub fn hits_path() -> PathBuf {
    support_dir().join("hits.jsonl")
}

pub fn append_hit(hit: &Hit) -> std::io::Result<()> {
    append_hit_to(&hits_path(), &LoggedHit {
        t: hit.t,
        phrase: hit.phrase.clone(),
    })
}

pub fn append_hit_to(path: &Path, hit: &LoggedHit) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(hit).unwrap())?;
    Ok(())
}

pub fn load_hits() -> Vec<LoggedHit> {
    load_hits_from(&hits_path())
}

pub fn load_hits_from(path: &Path) -> Vec<LoggedHit> {
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .filter_map(|line| line.ok())
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<LoggedHit>(&line).ok())
        .collect()
}

pub fn query_stats(
    hits: &[LoggedHit],
    start_ms: Option<i64>,
    end_ms: i64,
    phrase: Option<&str>,
    bucket: &str,
    tz_offset_min: i32,
) -> StatsResult {
    let start_ms = start_ms.unwrap_or_else(|| {
        hits.iter()
            .map(|h| secs_to_ms(h.t))
            .min()
            .unwrap_or(end_ms - DAY_MS)
    });
    let filtered: Vec<&LoggedHit> = hits
        .iter()
        .filter(|h| {
            let ms = secs_to_ms(h.t);
            ms >= start_ms && ms <= end_ms && phrase.map(|p| p == h.phrase).unwrap_or(true)
        })
        .collect();
    let mut phrase_set: Vec<String> = hits.iter().map(|h| h.phrase.clone()).collect();
    phrase_set.sort();
    phrase_set.dedup();

    let mut breakdown_map: HashMap<String, u32> = HashMap::new();
    for h in &filtered {
        *breakdown_map.entry(h.phrase.clone()).or_default() += 1;
    }
    let mut breakdown: Vec<PhraseCount> = breakdown_map
        .into_iter()
        .map(|(phrase, count)| PhraseCount { phrase, count })
        .collect();
    breakdown.sort_by(|a, b| b.count.cmp(&a.count).then(a.phrase.cmp(&b.phrase)));
    let total = filtered.iter().map(|_| 1u32).sum();

    let buckets = match bucket {
        "1w" => week_buckets(&filtered, start_ms, end_ms, tz_offset_min),
        "1mo" => month_buckets(&filtered, start_ms, end_ms, tz_offset_min),
        _ => fixed_buckets(&filtered, start_ms, end_ms, bucket_width(bucket)),
    };

    match buckets {
        Ok(buckets) => StatsResult {
            total,
            buckets,
            breakdown,
            phrases: phrase_set,
            too_many: None,
        },
        Err(too_many) => StatsResult {
            total,
            buckets: Vec::new(),
            breakdown,
            phrases: phrase_set,
            too_many: Some(too_many),
        },
    }
}

fn secs_to_ms(t: f64) -> i64 {
    (t * 1000.0).round() as i64
}

fn bucket_width(bucket: &str) -> i64 {
    match bucket {
        "15m" => 15 * MINUTE_MS,
        "1h" => HOUR_MS,
        _ => DAY_MS,
    }
}

fn fixed_buckets(
    hits: &[&LoggedHit],
    start_ms: i64,
    end_ms: i64,
    width: i64,
) -> Result<Vec<StatsBucket>, u32> {
    let aligned_start = start_ms.div_euclid(width) * width;
    let aligned_end = ((end_ms + width - 1).div_euclid(width)) * width;
    let count = ((aligned_end - aligned_start) / width).max(1);
    if count > MAX_BUCKETS {
        return Err(count as u32);
    }
    let mut maps: Vec<HashMap<String, u32>> = (0..count).map(|_| HashMap::new()).collect();
    for hit in hits {
        let idx = (secs_to_ms(hit.t) - aligned_start) / width;
        if idx >= 0 && idx < count {
            *maps[idx as usize].entry(hit.phrase.clone()).or_default() += 1;
        }
    }
    Ok(maps
        .into_iter()
        .enumerate()
        .map(|(i, map)| finish(aligned_start + i as i64 * width, map))
        .collect())
}

fn week_buckets(
    hits: &[&LoggedHit],
    start_ms: i64,
    end_ms: i64,
    tz_offset_min: i32,
) -> Result<Vec<StatsBucket>, u32> {
    let first = start_of_local_week(start_ms, tz_offset_min);
    let mut starts = Vec::new();
    let mut cursor = first;
    while cursor <= end_ms {
        starts.push(cursor);
        cursor += WEEK_MS;
    }
    if starts.is_empty() {
        starts.push(first);
    }
    fill_variable(hits, &starts, end_ms)
}

fn month_buckets(
    hits: &[&LoggedHit],
    start_ms: i64,
    end_ms: i64,
    tz_offset_min: i32,
) -> Result<Vec<StatsBucket>, u32> {
    let mut cursor = start_of_local_month(start_ms, tz_offset_min);
    let mut starts = Vec::new();
    while cursor <= end_ms {
        starts.push(cursor);
        cursor = add_local_month(cursor, tz_offset_min);
    }
    if starts.is_empty() {
        starts.push(start_of_local_month(start_ms, tz_offset_min));
    }
    fill_variable(hits, &starts, end_ms)
}

fn fill_variable(
    hits: &[&LoggedHit],
    starts: &[i64],
    end_ms: i64,
) -> Result<Vec<StatsBucket>, u32> {
    if starts.len() as i64 > MAX_BUCKETS {
        return Err(starts.len() as u32);
    }
    let mut maps: Vec<HashMap<String, u32>> = starts.iter().map(|_| HashMap::new()).collect();
    for hit in hits {
        let ms = secs_to_ms(hit.t);
        if let Some(i) = starts.iter().rposition(|s| ms >= *s && ms <= end_ms) {
            let end = starts.get(i + 1).copied().unwrap_or(end_ms + 1);
            if ms < end {
                *maps[i].entry(hit.phrase.clone()).or_default() += 1;
            }
        }
    }
    Ok(starts
        .iter()
        .zip(maps)
        .map(|(start, map)| finish(*start, map))
        .collect())
}

fn finish(start_ms: i64, map: HashMap<String, u32>) -> StatsBucket {
    let mut segments: Vec<PhraseCount> = map
        .into_iter()
        .map(|(phrase, count)| PhraseCount { phrase, count })
        .collect();
    segments.sort_by(|a, b| b.count.cmp(&a.count).then(a.phrase.cmp(&b.phrase)));
    let count = segments.iter().map(|s| s.count).sum();
    StatsBucket {
        start_ms,
        count,
        segments,
    }
}

fn shift_local(utc_ms: i64, tz_offset_min: i32) -> i64 {
    utc_ms - i64::from(tz_offset_min) * MINUTE_MS
}

fn unshift_local(local_ms: i64, tz_offset_min: i32) -> i64 {
    local_ms + i64::from(tz_offset_min) * MINUTE_MS
}

fn start_of_local_day(utc_ms: i64, tz_offset_min: i32) -> i64 {
    let local = shift_local(utc_ms, tz_offset_min);
    unshift_local(local.div_euclid(DAY_MS) * DAY_MS, tz_offset_min)
}

fn start_of_local_week(utc_ms: i64, tz_offset_min: i32) -> i64 {
    let day = start_of_local_day(utc_ms, tz_offset_min);
    let local_day = shift_local(day, tz_offset_min);
    let days = local_day.div_euclid(DAY_MS);
    let sun0 = ((days + 4).rem_euclid(7)) as i32;
    let back = i64::from((sun0 + 6) % 7);
    day - back * DAY_MS
}

fn start_of_local_month(utc_ms: i64, tz_offset_min: i32) -> i64 {
    let local = shift_local(utc_ms, tz_offset_min);
    let days = local.div_euclid(DAY_MS);
    let (y, m, _) = civil_from_days(days);
    let month_start_days = days_from_civil(y, m, 1);
    unshift_local(month_start_days * DAY_MS, tz_offset_min)
}

fn add_local_month(utc_ms: i64, tz_offset_min: i32) -> i64 {
    let local = shift_local(utc_ms, tz_offset_min);
    let days = local.div_euclid(DAY_MS);
    let (mut y, mut m, _) = civil_from_days(days);
    m += 1;
    if m == 13 {
        m = 1;
        y += 1;
    }
    unshift_local(days_from_civil(y, m, 1) * DAY_MS, tz_offset_min)
}

fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let mut y = y;
    let m = m as i32;
    y -= i32::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400) as u32;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d as i32 - 1;
    let doe = yoe as i64 * 365 + (yoe / 4 - yoe / 100) as i64 + i64::from(doy);
    i64::from(era) * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + i32::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(t: f64, phrase: &str) -> LoggedHit {
        LoggedHit {
            t,
            phrase: phrase.into(),
        }
    }

    #[test]
    fn jsonl_roundtrip() {
        let path = std::env::temp_dir().join(format!("pw-hits-{}.jsonl", std::process::id()));
        let _ = fs::remove_file(&path);
        append_hit_to(&path, &hit(1.0, "i'm sorry")).unwrap();
        append_hit_to(&path, &hit(2.0, "my bad")).unwrap();
        let loaded = load_hits_from(&path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[1].phrase, "my bad");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn filters_and_buckets() {
        let hits = vec![
            hit(1_000.0, "sorry"),
            hit(1_800.0, "sorry"),
            hit(4_000.0, "my bad"),
        ];
        let stats = query_stats(&hits, Some(0), 5_000_000, Some("sorry"), "1h", 0);
        assert_eq!(stats.total, 2);
        assert_eq!(stats.breakdown.len(), 1);
        assert_eq!(stats.breakdown[0].count, 2);
        let hour_counts: u32 = stats.buckets.iter().map(|b| b.count).sum();
        assert_eq!(hour_counts, 2);
    }

    #[test]
    fn all_phrases_stack() {
        let hits = vec![hit(10.0, "a"), hit(11.0, "b"), hit(12.0, "a")];
        let stats = query_stats(&hits, Some(0), 20_000, None, "15m", 0);
        assert_eq!(stats.total, 3);
        assert_eq!(stats.breakdown[0].phrase, "a");
        assert_eq!(stats.breakdown[0].count, 2);
    }
}
