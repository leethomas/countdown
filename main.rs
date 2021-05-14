extern crate clap;
extern crate rand;

use std::convert::TryFrom;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
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


#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct Event {
  name: String,
  // Unix timestamp (seconds)
  time: u32,
}

// Validated event that has definitley not occurred yet.
struct FutureEvent {
  name: String,
  days_left: u16,
}

impl Event {
  fn days_left(&self, current_time: SystemTime) -> Result<u16, String> {
    match self.system_time().duration_since(current_time) {
      Ok(dur) => u16::try_from(dur.as_secs() / SECONDS_IN_DAY)
        .map_err(|e| format!("Error calculating days between: {:?}", e)),
      Err(e) => Err(format!("{:?}", e)),
    }
  }

  // duration_since will return an error if the duration is negative, indicating
  // that the event has passed.
  fn has_passed(&self, current_time: SystemTime) -> bool {
    self.system_time()
      .duration_since(current_time)
      .ok()
      .is_none()
  }

  // TODO: make this a trait, impl From on FutureEvent?
  fn as_future_event(&self, current_time: SystemTime) -> Option<FutureEvent> {
    self.days_left(current_time).ok().map(|days| FutureEvent {
      name: self.name.clone(),
      days_left: days,
    })
  }

  fn system_time(&self) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(self.time.into())
  }
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
    .map(|config: CountdownConfig|
      applicable_events(now, config.events, &cli_matches)
        .iter()
        .map(|ev|
          format!("{} days until {}", ev.days_left, ev.name)
        ).collect()
    );

  match result  {
      Ok(events) => {
        events.iter().for_each(|msg| println!("{}", msg))
      },
      Err(e) => eprintln!("{:?}", e),
    }
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

fn events_sorted_by_date(events: &Vec<Event>, is_asc: bool) -> Vec<Event> {
  let mut cloned_events = events.clone();
  cloned_events
    .sort_by(|a, b| if is_asc {
      a.time.cmp(&b.time)
    } else {
      b.time.cmp(&a.time)
    });

  cloned_events
}

// TODO: Maybe this should return a Result in case of unsupported sort types.
fn sort_events(events: &Vec<Event>, cli_args: &clap::ArgMatches) -> Vec<Event> {
  match cli_args.value_of(ARG_ORDER).map(|order| {
    if order == ARG_ORDER_SHUFFLE {
      let mut cloned = events.clone();
      cloned.shuffle(&mut thread_rng());
      cloned
    } else if order == ARG_ORDER_TIME_DESC {
      events_sorted_by_date(events, false)
    } else if order == ARG_ORDER_TIME_ASC {
      events_sorted_by_date(events, true)
    } else {
      events_sorted_by_date(events, true)
    }
  }) {
    Some(results) => results,
    None => events_sorted_by_date(events, true),
  }
}

fn limit_events(events: &Vec<Event>, cli_args: &clap::ArgMatches) -> Vec<Event> {
  let cloned_events = events.clone().into_iter();

  match cli_args.value_of(ARG_LIST_N)
    .and_then(|n| n.parse::<usize>().ok()) {
      Some(limit) => cloned_events
        .take(limit)
        .collect(),
      None => cloned_events.collect(),
    }
}

fn applicable_events(
  now: SystemTime,
  events: Vec<Event>,
  cli_args: &clap::ArgMatches,
) -> Vec<FutureEvent> {
  let current = filter_expired_events(now, &events);
  let sorted = sort_events(&current, cli_args);
  let limited = limit_events(&sorted, cli_args);

  limited
    .into_iter()
    .filter_map(|ev| ev.as_future_event(now))
    .collect()
}
