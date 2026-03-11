use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Duration, LocalResult, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub enum Schedule {
    Always,
    Window {
        timezone: Tz,
        start: NaiveTime,
        end: NaiveTime,
    },
}

#[derive(Debug, Clone)]
pub enum WindowState {
    Always,
    Active {
        window_end: DateTime<Utc>,
        timezone: Tz,
    },
    Inactive {
        next_start: DateTime<Utc>,
        timezone: Tz,
    },
}

impl Schedule {
    pub fn from_env(
        active_start: Option<&str>,
        active_end: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<Self> {
        match (active_start, active_end) {
            (None, None) => Ok(Self::Always),
            (Some(_), None) | (None, Some(_)) => bail!(
                "DISCORD_ACTIVE_START and DISCORD_ACTIVE_END must either both be set or both be unset"
            ),
            (Some(start), Some(end)) => {
                let start = parse_time(start, "DISCORD_ACTIVE_START")?;
                let end = parse_time(end, "DISCORD_ACTIVE_END")?;
                if start == end {
                    bail!("DISCORD_ACTIVE_START and DISCORD_ACTIVE_END cannot be the same time");
                }

                let timezone = timezone.unwrap_or("UTC").parse::<Tz>().map_err(|_| {
                    anyhow!(
                        "Invalid DISCORD_TIMEZONE '{}'. Use an IANA timezone such as 'America/Chicago'",
                        timezone.unwrap_or("UTC")
                    )
                })?;

                Ok(Self::Window {
                    timezone,
                    start,
                    end,
                })
            }
        }
    }

    pub fn state_at(&self, now_utc: DateTime<Utc>) -> Result<WindowState> {
        match self {
            Self::Always => Ok(WindowState::Always),
            Self::Window {
                timezone,
                start,
                end,
            } => {
                let local_now = now_utc.with_timezone(timezone);
                let today = local_now.date_naive();
                let time = local_now.time();

                if start < end {
                    if time >= *start && time < *end {
                        let window_end = local_to_utc(*timezone, today, *end)?;
                        Ok(WindowState::Active {
                            window_end,
                            timezone: *timezone,
                        })
                    } else {
                        let next_day = if time < *start {
                            today
                        } else {
                            today + Duration::days(1)
                        };
                        let next_start = local_to_utc(*timezone, next_day, *start)?;
                        Ok(WindowState::Inactive {
                            next_start,
                            timezone: *timezone,
                        })
                    }
                } else if time >= *start || time < *end {
                    let end_day = if time < *end {
                        today
                    } else {
                        today + Duration::days(1)
                    };
                    let window_end = local_to_utc(*timezone, end_day, *end)?;
                    Ok(WindowState::Active {
                        window_end,
                        timezone: *timezone,
                    })
                } else {
                    let next_start = local_to_utc(*timezone, today, *start)?;
                    Ok(WindowState::Inactive {
                        next_start,
                        timezone: *timezone,
                    })
                }
            }
        }
    }
}

fn parse_time(value: &str, name: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M:%S"))
        .map_err(|_| {
            anyhow!(
                "Invalid {} '{}'. Use 24-hour time like 09:30 or 21:45",
                name,
                value
            )
        })
}

fn local_to_utc(timezone: Tz, date: chrono::NaiveDate, time: NaiveTime) -> Result<DateTime<Utc>> {
    let naive = date.and_time(time);

    let local = match timezone.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(first, second) => first.min(second),
        LocalResult::None => {
            let mut probe = naive + Duration::minutes(1);
            let limit = naive + Duration::hours(3);

            loop {
                match timezone.from_local_datetime(&probe) {
                    LocalResult::Single(dt) => break dt,
                    LocalResult::Ambiguous(first, second) => break first.min(second),
                    LocalResult::None if probe < limit => {
                        probe += Duration::minutes(1);
                    }
                    LocalResult::None => {
                        bail!(
                            "Could not resolve local time {} in timezone {}",
                            naive,
                            timezone
                        );
                    }
                }
            }
        }
    };

    Ok(local.with_timezone(&Utc))
}
