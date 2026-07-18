use rnga::models::TopicOrder;
use rust_i18n::t;

pub(super) fn parse_order(order: &str) -> TopicOrder {
    match order {
        "postdate" => TopicOrder::PostDate,
        "recommend" => TopicOrder::Recommend,
        _ => TopicOrder::LastPost,
    }
}

pub(super) fn effective_concurrency(requested: usize) -> usize {
    let max_concurrency = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(8);
    requested.min(max_concurrency).max(1)
}

pub fn parse_time_range(range: &str) -> Option<(i64, String)> {
    let range_lower = range.to_lowercase();

    if range_lower.len() >= 2 {
        let (num_str, unit) = range_lower.split_at(range_lower.len() - 1);
        if let Ok(num) = num_str.parse::<i64>() {
            match unit {
                "s" => {
                    let unit_str = if num != 1 {
                        t!("seconds")
                    } else {
                        t!("second")
                    };
                    return Some((num, format!("{} {}", num, unit_str)));
                }
                "m" => {
                    let unit_str = if num != 1 {
                        t!("minutes")
                    } else {
                        t!("minute")
                    };
                    return Some((num * 60, format!("{} {}", num, unit_str)));
                }
                "h" => {
                    let unit_str = if num != 1 { t!("hours") } else { t!("hour") };
                    return Some((num * 3600, format!("{} {}", num, unit_str)));
                }
                "d" => {
                    let unit_str = if num != 1 { t!("days") } else { t!("day") };
                    return Some((num * 86400, format!("{} {}", num, unit_str)));
                }
                _ => {}
            }
        }
    }

    match range_lower.as_str() {
        "second" | "1s" => Some((1, t!("second").to_string())),
        "minute" | "1m" => Some((60, t!("minute").to_string())),
        "hour" | "1h" => Some((3600, t!("hour").to_string())),
        "day" | "1d" => Some((86400, t!("day").to_string())),
        "week" => Some((604800, t!("week").to_string())),
        "month" => Some((2592000, t!("month").to_string())),
        "year" => Some((31536000, t!("year").to_string())),
        _ => None,
    }
}
