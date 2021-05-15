extern crate clap;
extern crate rand;

use std::convert::TryFrom;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rand::thread_rng;
use rand::seq::SliceRandom;

const SECONDS_IN_DAY: u64 = 86400;
const CONFIG_FILENAME: &str = ".countdown.yml";
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
  // Unix timestamp (seconds)
  time: u32,
}

impl Event {
  fn days_left(&self, current_time: SystemTime) -> Option<u16> {
    self.system_time()
      .duration_since(current_time)
      .ok()
      .and_then(|dur| {
        u16::try_from(dur.as_secs() / SECONDS_IN_DAY).ok()
      })
  }

  // duration_since will return Ok() if the event time is in the future
  fn has_passed(&self, current_time: SystemTime) -> bool {
    self.system_time()
      .duration_since(current_time)
      .ok()
      .is_none()
  }

  // TODO: make this a trait, impl From on FutureEvent?
  fn as_future_event(&self, current_time: SystemTime) -> Option<FutureEvent> {
    self.days_left(current_time).map(|days| FutureEvent {
      name: self.name.clone(),
      days_left: days,
    })
  }

  fn system_time(&self) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(self.time.into())
  }
}

// Validated event that has definitley not occurred yet.
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
  let now = SystemTime::now();
  let cli_config = clap::load_yaml!("cli.yml");
  let cli_matches = clap::App::from_yaml(cli_config).get_matches();
  let result: Result<Vec<String>, String> = dirs::home_dir()
    .ok_or_else(|| "Failed to find home directory.".to_string())
    .map(|home| [home, CONFIG_FILENAME.into()].iter().collect::<PathBuf>())
    .and_then(|config_file| confy::load_path(config_file)
      .map_err(|e| format!("Couldn't load config: {:?}", e).to_string()))
    .and_then(|config: CountdownConfig|
      collect_args(&cli_matches).map(|args|
        applicable_events(now, config.events, &args)
          .iter()
          .map(|ev|
            format!("{} days until {}", ev.days_left, ev.name)
          ).collect()
      )
    );

  match result  {
      Ok(events) => {
        events.iter().for_each(|msg| println!("{}", msg))
      },
      Err(e) => eprintln!("{:?}", e),
    }
}

fn collect_args(clap_args: &clap::ArgMatches)
  -> Result<CountdownArgs, String> {
    // Largely uneeded because of Clap's validation, but
    // it's nice to have.
  let order = match clap_args.value_of(ARG_ORDER) {
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

fn filter_expired_events(now: SystemTime, events: &Vec<Event>) -> Vec<Event> {
  events
    .clone()
    .into_iter()
    .filter_map(|ev| if ev.has_passed(now) {
        None
     } else {
       Some(ev)
     })
    .collect()
}

fn events_sorted_by_time(events: &Vec<Event>, is_asc: bool) -> Vec<Event> {
  let mut cloned_events = events.clone();
  cloned_events
    .sort_by(|a, b| if is_asc {
      a.time.cmp(&b.time)
    } else {
      b.time.cmp(&a.time)
    });

  cloned_events
}

fn sort_events(events: &Vec<Event>, args: &CountdownArgs) -> Vec<Event> {
  match &args.order {
    Some(order) => match order {
      SortOrder::Shuffle => {
        let mut cloned = events.clone();
        cloned.shuffle(&mut thread_rng());

        cloned
      },
      SortOrder::TimeAsc => {
        events_sorted_by_time(events, true)
      },
      SortOrder::TimeDesc => {
        events_sorted_by_time(events, false)
      },
    },
    None => events_sorted_by_time(events, true)
  }
}

fn limit_events(events: Vec<Event>, args: &CountdownArgs) -> Vec<Event> {
  match args.n {
    Some(limit) => events.into_iter().take(limit).collect(),
    None => events,
  }
}

fn applicable_events(
  now: SystemTime,
  events: Vec<Event>,
  args: &CountdownArgs,
) -> Vec<FutureEvent> {
  let current = filter_expired_events(now, &events);
  let sorted = sort_events(&current, &args);
  let limited = limit_events(sorted, args);

  limited
    .into_iter()
    .filter_map(|ev| ev.as_future_event(now))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  // Event
  #[test]
  fn event_has_passed_is_true_when_event_expires() {
    let event = Event { name: "expired".to_string(), time: 10 };
    assert!(event.has_passed(UNIX_EPOCH + Duration::from_secs(11)));
  }

  // other functions
  #[test]
  fn filter_expired_events_removes_expired_events() {
    let events = vec![
      Event { name: "expired 1".to_string(), time: 900 },
      Event { name: "not expired 1".to_string(), time: 1020 },
      Event { name: "expired 3".to_string(), time: 543 },
    ];
    let result = filter_expired_events(
      UNIX_EPOCH + Duration::from_secs(1000),
      &events,
    );

    assert_eq!(
      result,
      vec![Event { name: "not expired 1".to_string(), time: 1020 }],
    );
  }

  #[test]
  fn events_sorted_by_time_sorts_in_asc_order() {
    let events = vec![
      Event { name: "test 1".to_string(), time: 900 },
      Event { name: "test 2".to_string(), time: 1020 },
      Event { name: "test 3".to_string(), time: 543 },
    ];
    let result = events_sorted_by_time(&events, true);
    
    assert_eq!(
      result,
      vec![
        Event { name: "test 3".to_string(), time: 543 },
        Event { name: "test 1".to_string(), time: 900 },
        Event { name: "test 2".to_string(), time: 1020 },
      ],
    );
  }

  #[test]
  fn events_sorted_by_time_sorts_in_desc_order() {
    let events = vec![
      Event { name: "test 1".to_string(), time: 900 },
      Event { name: "test 2".to_string(), time: 1020 },
      Event { name: "test 3".to_string(), time: 543 },
    ];
    let result = events_sorted_by_time(&events, false);
    
    assert_eq!(
      result,
      vec![
        Event { name: "test 2".to_string(), time: 1020 },
        Event { name: "test 1".to_string(), time: 900 },
        Event { name: "test 3".to_string(), time: 543 },
      ],
    );
  }
}