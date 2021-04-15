use std::convert::TryFrom;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SECONDS_IN_DAY: u64 = 86400;
const K_EVENT_NAME: &str = "name";
const K_EVENT_TIME: &str = "time";
const K_CONFIG_CURRENT_EVENT: &str = "current-event";

// TODO: enums
const E_CONFIG_NOT_FOUND: &str = "Couldn't load config.";
const E_EVENT_NAME_NOT_FOUND: &str = "Could not load event name.";
const E_EVENT_TIME_NOT_FOUND: &str = "Could not load event time.";

fn main() {
  match get_config(config::Config::default()).and_then(|conf| run(&conf)) {
    Ok(res) => print!("{}", res.to_string()),
    Err(e) => {
      eprintln!("countdown: {}", e)
    }
  }
}

fn get_config(conf: config::Config) -> Result<config::Config, String> {
  conf
    .with_merged(config::File::with_name("/Users/lee.thomas/.countdown"))
    .map_err(|_| "Error parsing config.".to_string())
}

fn run(conf: &config::Config) -> Result<String, String> {
  let parsed_config = conf
    .get_table(K_CONFIG_CURRENT_EVENT)
    .map_err(|_| E_CONFIG_NOT_FOUND)?;
  let event_name = parsed_config
    .get(K_EVENT_NAME)
    .ok_or_else(|| E_EVENT_NAME_NOT_FOUND)
    // TODO: Need enum for this, this is not the right error.
    .and_then(|v| v.clone().into_str().map_err(|_| E_EVENT_NAME_NOT_FOUND))?;
  let event_time = parsed_config
    .get(K_EVENT_TIME)
    .ok_or_else(|| E_EVENT_TIME_NOT_FOUND)
    // TODO: Need enum for this, this is not the right error.
    .and_then(|v| v.clone().into_int().map_err(|_| E_EVENT_TIME_NOT_FOUND))
    .and_then(|num| u64::try_from(num).map_err(|_| "boom"))?;
  let days_left = days_between(SystemTime::now(), Duration::from_secs(event_time))?;

  Ok(format!("| {} days until {}!", days_left, event_name))
}

fn days_between(now: SystemTime, future_offset_from_unix_time: Duration) -> Result<u64, String> {
  let future_time = UNIX_EPOCH + future_offset_from_unix_time;

  match future_time.duration_since(now) {
    Ok(dur) => Ok(dur.as_secs() / SECONDS_IN_DAY),
    Err(e) => Err(format!("{:?}", e)),
  }
}
