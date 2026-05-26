extern crate chrono;
extern crate clap;
extern crate rand;

use rand::seq::SliceRandom;
use rand::thread_rng;
use std::path::Path;

const CONFIG_FILENAME: &str = ".countdown.toml";
const ARG_LIST_N: &str = "n";
const ARG_ORDER: &str = "order";
const ARG_ORDER_SHUFFLE: &str = "shuffle";
const ARG_ORDER_TIME_DESC: &str = "time-desc";
const ARG_ORDER_TIME_ASC: &str = "time-asc";

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct CountdownConfig {
    events: Vec<Event>,
}

impl Default for CountdownConfig {
    fn default() -> Self {
        Self { events: Vec::new() }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
struct Event {
    name: String,
    date: chrono::NaiveDate,
}

impl Event {
    /// Returns the number of days until the event if the event is either today or in the future, otherwise returns None.
    fn days_left(&self, current_time: chrono::DateTime<chrono::Local>) -> Option<u16> {
        let days = self
            .date
            .signed_duration_since(current_time.date_naive())
            .num_days();

        if days >= 0 && days <= u16::MAX as i64 {
            Some(days as u16)
        } else {
            None
        }
    }

    fn as_future_event(
        &self,
        current_time: chrono::DateTime<chrono::Local>,
    ) -> Option<FutureEvent> {
        self.days_left(current_time).map(|days| FutureEvent {
            name: self.name.clone(),
            days_left: days,
        })
    }
}

// Validated event that has definitely not occurred yet.
#[derive(Debug, Clone, PartialEq)]
struct FutureEvent {
    name: String,
    days_left: u16,
}

enum SortOrder {
    Shuffle,
    TimeAsc,
    TimeDesc,
}

impl std::str::FromStr for SortOrder {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            ARG_ORDER_SHUFFLE => Ok(Self::Shuffle),
            ARG_ORDER_TIME_ASC => Ok(Self::TimeAsc),
            ARG_ORDER_TIME_DESC => Ok(Self::TimeDesc),
            _ => Err(format!("Invalid value for 'order': {}", s)),
        }
    }
}

struct CountdownArgs {
    order: Option<SortOrder>,
    n: Option<usize>,
}

fn main() {
    let now = chrono::Local::now();
    let cli_config = clap::load_yaml!("cli.yml");
    let cli_matches = clap::App::from_yaml(cli_config).get_matches();
    let result: Result<Vec<FutureEvent>, String> = dirs::home_dir()
        .ok_or_else(|| "Failed to find home directory.".to_string())
        .map(|home| home.join(Path::new(CONFIG_FILENAME)))
        .and_then(|config_file| {
            confy::load_path(config_file)
                .map_err(|e| format!("Couldn't load config: {:?}", e).to_string())
        })
        .and_then(|config: CountdownConfig| {
            collect_args(&cli_matches).map(|args| applicable_events(now, config.events, &args))
        });

    match result {
        Ok(events) => events
            .iter()
            .for_each(|ev| println!("{}d until {}", ev.days_left, ev.name)),
        Err(e) => eprintln!("{:?}", e),
    }
}

fn collect_args(clap_args: &clap::ArgMatches) -> Result<CountdownArgs, String> {
    // Largely unneeded because of Clap's validation, but
    // it's nice to have.
    let order: Option<SortOrder> = match clap_args.value_of(ARG_ORDER) {
        Some(o) => o.parse::<SortOrder>().map(Some),
        None => Ok(None),
    }?;

    let n = match clap_args.value_of(ARG_LIST_N) {
        Some(limit) => limit
            .parse::<usize>()
            .map(Some)
            .map_err(|e| format!("Error parsing 'n': {:?}", e)),
        None => Ok(None),
    }?;

    Ok(CountdownArgs { order, n })
}

fn filter_expired_events(
    now: chrono::DateTime<chrono::Local>,
    events: &Vec<Event>,
) -> Vec<FutureEvent> {
    events
        .iter()
        .filter_map(|ev| ev.as_future_event(now))
        .collect()
}

fn events_sorted_by_time(events: &Vec<FutureEvent>, is_asc: bool) -> Vec<FutureEvent> {
    let mut cloned_events = events.clone();
    cloned_events.sort_by(|a, b| {
        if is_asc {
            a.days_left.cmp(&b.days_left)
        } else {
            b.days_left.cmp(&a.days_left)
        }
    });

    cloned_events
}

fn sort_events(events: &Vec<FutureEvent>, order: &Option<SortOrder>) -> Vec<FutureEvent> {
    match order {
        Some(o) => match o {
            SortOrder::Shuffle => {
                let mut cloned = events.clone();
                cloned.shuffle(&mut thread_rng());

                cloned
            }
            SortOrder::TimeAsc => events_sorted_by_time(events, true),
            SortOrder::TimeDesc => events_sorted_by_time(events, false),
        },
        None => events_sorted_by_time(events, true),
    }
}

fn limit_events(events: Vec<FutureEvent>, limit: Option<usize>) -> Vec<FutureEvent> {
    match limit {
        Some(n) => events.into_iter().take(n).collect(),
        None => events,
    }
}

fn applicable_events(
    now: chrono::DateTime<chrono::Local>,
    events: Vec<Event>,
    args: &CountdownArgs,
) -> Vec<FutureEvent> {
    let current = filter_expired_events(now, &events);
    let sorted = sort_events(&current, &args.order);

    limit_events(sorted, args.n)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn local_dt(year: i32, month: u32, day: u32) -> chrono::DateTime<chrono::Local> {
        chrono::Local
            .with_ymd_and_hms(year, month, day, 0, 0, 0)
            .single()
            .unwrap()
    }

    // Event
    #[test]
    fn event_days_left_calculates_remaining_days_correctly() {
        let today = local_dt(2026, 1, 1);
        let event_dt = today.checked_add_days(chrono::Days::new(2)).unwrap();
        let event = Event {
            name: "test".to_string(),
            date: event_dt.date_naive(),
        };

        let result = event.days_left(today.with_timezone(&chrono::Local));

        assert_eq!(result, Some(2));
    }

    #[test]
    fn event_days_left_returns_none_if_expired() {
        let today = local_dt(2026, 1, 8);
        let event_dt = local_dt(2026, 1, 7);
        let event = Event {
            name: "test".to_string(),
            date: event_dt.date_naive(),
        };
        let result = event.days_left(today);

        assert_eq!(result, None);
    }

    #[test]
    fn event_as_future_event_returns_future_event_if_not_expired() {
        let event_dt = chrono::DateTime::UNIX_EPOCH
            .checked_add_days(chrono::Days::new(2))
            .map(|d| d.with_timezone(&chrono::Local))
            .unwrap();
        let event = Event {
            name: "test".to_string(),
            date: event_dt.date_naive(),
        };
        let result =
            event.as_future_event(chrono::DateTime::UNIX_EPOCH.with_timezone(&chrono::Local));

        assert_eq!(
            result,
            Some(FutureEvent {
                name: "test".to_string(),
                days_left: 2,
            })
        );
    }

    #[test]
    fn event_as_future_event_returns_none_if_expired() {
        let event_dt = chrono::DateTime::UNIX_EPOCH.with_timezone(&chrono::Local);
        let current_dt = chrono::DateTime::UNIX_EPOCH
            .checked_add_days(chrono::Days::new(1))
            .map(|d| d.with_timezone(&chrono::Local))
            .unwrap();
        let event = Event {
            name: "test".to_string(),
            date: event_dt.date_naive(),
        };

        let result = event.as_future_event(current_dt);

        assert_eq!(result, None);
    }

    #[test]
    fn filter_expired_events_removes_expired_events() {
        let current_dt = local_dt(2026, 1, 10);
        let not_expired_dt = local_dt(2026, 1, 15);
        let expired_dt_1 = local_dt(2026, 1, 5);
        let expired_dt_2 = local_dt(2026, 1, 3);

        let events = vec![
            Event {
                name: "expired 1".to_string(),
                date: expired_dt_1.date_naive(),
            },
            Event {
                name: "not expired 1".to_string(),
                date: not_expired_dt.date_naive(),
            },
            Event {
                name: "expired 3".to_string(),
                date: expired_dt_2.date_naive(),
            },
        ];
        let result = filter_expired_events(current_dt, &events);

        assert_eq!(
            result,
            vec![FutureEvent {
                name: "not expired 1".to_string(),
                days_left: 5
            }],
        );
    }

    #[test]
    fn sort_events_sorts_in_asc_order() {
        let events = vec![
            FutureEvent {
                name: "test 1".to_string(),
                days_left: 900,
            },
            FutureEvent {
                name: "test 2".to_string(),
                days_left: 1020,
            },
            FutureEvent {
                name: "test 3".to_string(),
                days_left: 543,
            },
        ];
        let result = sort_events(&events, &Some(SortOrder::TimeAsc));

        assert_eq!(
            result,
            vec![
                FutureEvent {
                    name: "test 3".to_string(),
                    days_left: 543
                },
                FutureEvent {
                    name: "test 1".to_string(),
                    days_left: 900
                },
                FutureEvent {
                    name: "test 2".to_string(),
                    days_left: 1020
                },
            ],
        );
    }

    #[test]
    fn sort_events_sorts_in_desc_order() {
        let events = vec![
            FutureEvent {
                name: "test 1".to_string(),
                days_left: 900,
            },
            FutureEvent {
                name: "test 2".to_string(),
                days_left: 1020,
            },
            FutureEvent {
                name: "test 3".to_string(),
                days_left: 543,
            },
        ];
        let result = sort_events(&events, &Some(SortOrder::TimeDesc));

        assert_eq!(
            result,
            vec![
                FutureEvent {
                    name: "test 2".to_string(),
                    days_left: 1020
                },
                FutureEvent {
                    name: "test 1".to_string(),
                    days_left: 900
                },
                FutureEvent {
                    name: "test 3".to_string(),
                    days_left: 543
                },
            ],
        );
    }
}
