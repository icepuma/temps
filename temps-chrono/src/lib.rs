use chrono::{DateTime, Datelike, Duration, Local, Months};
use temps_core::{
    DayReference, Direction, Language, TimeExpression, TimeParser, TimeUnit, Weekday,
    constants::MONTHS_PER_YEAR,
    errors::*,
    time_utils::{
        calculate_timezone_offset_seconds, calculate_weekday_offset, convert_12_to_24_hour,
    },
};

pub struct ChronoProvider;

impl TimeParser for ChronoProvider {
    type DateTime = DateTime<Local>;
    type Error = String;

    fn now(&self) -> Self::DateTime {
        Local::now()
    }

    fn parse_expression(&self, expr: TimeExpression) -> Result<Self::DateTime, Self::Error> {
        match expr {
            TimeExpression::Now => Ok(self.now()),
            TimeExpression::Relative(rel) => {
                let now = self.now();

                // Handle months and years separately for proper date arithmetic
                match rel.unit {
                    TimeUnit::Month => {
                        let months = Months::new(
                            rel.amount
                                .try_into()
                                .map_err(|_| ERR_MONTH_POSITIVE.to_string())?,
                        );

                        match rel.direction {
                            Direction::Past => now
                                .checked_sub_months(months)
                                .ok_or_else(|| ERR_DATE_CALC_INVALID.to_string()),
                            Direction::Future => now
                                .checked_add_months(months)
                                .ok_or_else(|| ERR_DATE_CALC_INVALID.to_string()),
                        }
                    }
                    TimeUnit::Year => {
                        // Convert years to months for proper arithmetic
                        let months_count = rel
                            .amount
                            .checked_mul(MONTHS_PER_YEAR as i64)
                            .ok_or_else(|| ERR_YEAR_OVERFLOW.to_string())?;
                        let months = Months::new(
                            months_count
                                .try_into()
                                .map_err(|_| ERR_YEAR_POSITIVE.to_string())?,
                        );

                        match rel.direction {
                            Direction::Past => now
                                .checked_sub_months(months)
                                .ok_or_else(|| ERR_DATE_CALC_INVALID.to_string()),
                            Direction::Future => now
                                .checked_add_months(months)
                                .ok_or_else(|| ERR_DATE_CALC_INVALID.to_string()),
                        }
                    }
                    _ => {
                        // Use Duration for time units that have fixed lengths
                        let duration = match rel.unit {
                            TimeUnit::Second => Duration::seconds(rel.amount),
                            TimeUnit::Minute => Duration::minutes(rel.amount),
                            TimeUnit::Hour => Duration::hours(rel.amount),
                            TimeUnit::Day => Duration::days(rel.amount),
                            TimeUnit::Week => Duration::weeks(rel.amount),
                            _ => unreachable!(), // Month and Year handled above
                        };

                        match rel.direction {
                            Direction::Past => Ok(now - duration),
                            Direction::Future => Ok(now + duration),
                        }
                    }
                }
            }
            TimeExpression::Absolute(abs) => {
                use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

                let date =
                    NaiveDate::from_ymd_opt(abs.year as i32, abs.month as u32, abs.day as u32)
                        .ok_or_else(|| format_invalid_date(abs.year, abs.month, abs.day))?;

                let datetime = if let (Some(hour), Some(minute)) = (abs.hour, abs.minute) {
                    let time = NaiveTime::from_hms_nano_opt(
                        hour as u32,
                        minute as u32,
                        abs.second.unwrap_or(0) as u32,
                        abs.nanosecond.unwrap_or(0),
                    )
                    .ok_or_else(|| {
                        format!(
                            "Invalid time: {}:{}:{}",
                            hour,
                            minute,
                            abs.second.unwrap_or(0)
                        )
                    })?;

                    let naive_dt = NaiveDateTime::new(date, time);

                    match &abs.timezone {
                        Some(temps_core::Timezone::Utc) => {
                            Utc.from_utc_datetime(&naive_dt).with_timezone(&Local)
                        }
                        Some(temps_core::Timezone::Offset { hours, minutes }) => {
                            let offset_seconds =
                                calculate_timezone_offset_seconds(*hours, *minutes);
                            let offset = FixedOffset::east_opt(offset_seconds)
                                .ok_or_else(|| format_invalid_timezone_offset(*hours, *minutes))?;
                            offset
                                .from_local_datetime(&naive_dt)
                                .single()
                                .ok_or_else(|| ERR_AMBIGUOUS_TIME.to_string())?
                                .with_timezone(&Local)
                        }
                        None => {
                            // No timezone specified, treat as local time
                            Local
                                .from_local_datetime(&naive_dt)
                                .single()
                                .ok_or_else(|| ERR_AMBIGUOUS_TIME.to_string())?
                        }
                    }
                } else {
                    // Date only, set time to midnight
                    let midnight = date
                        .and_hms_opt(0, 0, 0)
                        .ok_or_else(|| ERR_MIDNIGHT_FAILED.to_string())?;
                    Local
                        .from_local_datetime(&midnight)
                        .single()
                        .ok_or_else(|| "Ambiguous or invalid local time".to_string())?
                };

                Ok(datetime)
            }
            TimeExpression::Day(day_ref) => {
                let now = self.now();
                match day_ref {
                    DayReference::Today => {
                        let midnight = now
                            .date_naive()
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| ERR_MIDNIGHT_FAILED.to_string())?;
                        midnight
                            .and_local_timezone(Local)
                            .single()
                            .ok_or_else(|| "Ambiguous or invalid local time".to_string())
                    }
                    DayReference::Yesterday => {
                        let yesterday = now - Duration::days(1);
                        let midnight = yesterday
                            .date_naive()
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| ERR_MIDNIGHT_FAILED.to_string())?;
                        midnight
                            .and_local_timezone(Local)
                            .single()
                            .ok_or_else(|| "Ambiguous or invalid local time".to_string())
                    }
                    DayReference::Tomorrow => {
                        let tomorrow = now + Duration::days(1);
                        let midnight = tomorrow
                            .date_naive()
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| ERR_MIDNIGHT_FAILED.to_string())?;
                        midnight
                            .and_local_timezone(Local)
                            .single()
                            .ok_or_else(|| "Ambiguous or invalid local time".to_string())
                    }
                    DayReference::Weekday { day, modifier } => {
                        let target_weekday = match day {
                            Weekday::Monday => chrono::Weekday::Mon,
                            Weekday::Tuesday => chrono::Weekday::Tue,
                            Weekday::Wednesday => chrono::Weekday::Wed,
                            Weekday::Thursday => chrono::Weekday::Thu,
                            Weekday::Friday => chrono::Weekday::Fri,
                            Weekday::Saturday => chrono::Weekday::Sat,
                            Weekday::Sunday => chrono::Weekday::Sun,
                        };

                        let current_weekday = now.weekday();
                        let current_offset = current_weekday.num_days_from_monday() as i64;
                        let target_offset = target_weekday.num_days_from_monday() as i64;

                        let days_to_add =
                            calculate_weekday_offset(current_offset, target_offset, modifier);
                        let target_date = now + Duration::days(days_to_add);

                        let midnight = target_date
                            .date_naive()
                            .and_hms_opt(0, 0, 0)
                            .ok_or_else(|| ERR_MIDNIGHT_FAILED.to_string())?;
                        midnight
                            .and_local_timezone(Local)
                            .single()
                            .ok_or_else(|| "Ambiguous or invalid local time".to_string())
                    }
                }
            }
            TimeExpression::Time(time) => {
                let now = self.now();
                let hour = convert_12_to_24_hour(time.hour, time.meridiem.as_ref()) as u32;

                Ok(now
                    .date_naive()
                    .and_hms_opt(hour, time.minute as u32, time.second as u32)
                    .ok_or_else(|| "Invalid time".to_string())?
                    .and_local_timezone(Local)
                    .single()
                    .ok_or_else(|| "Ambiguous local time".to_string())?)
            }
            TimeExpression::DayTime(day_time) => {
                // First get the day
                let day_result =
                    self.parse_expression(TimeExpression::Day(day_time.day.clone()))?;
                let date = day_result.date_naive();

                let hour =
                    convert_12_to_24_hour(day_time.time.hour, day_time.time.meridiem.as_ref())
                        as u32;

                Ok(date
                    .and_hms_opt(
                        hour,
                        day_time.time.minute as u32,
                        day_time.time.second as u32,
                    )
                    .ok_or_else(|| "Invalid time".to_string())?
                    .and_local_timezone(Local)
                    .single()
                    .ok_or_else(|| "Ambiguous local time".to_string())?)
            }
            TimeExpression::Date(date) => {
                use chrono::NaiveDate;

                NaiveDate::from_ymd_opt(date.year as i32, date.month as u32, date.day as u32)
                    .ok_or_else(|| "Invalid date".to_string())?
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| "Invalid datetime".to_string())?
                    .and_local_timezone(Local)
                    .single()
                    .ok_or_else(|| "Ambiguous local time".to_string())
            }
        }
    }
}

pub fn parse_to_datetime(input: &str, language: Language) -> Result<DateTime<Local>, String> {
    let expr = temps_core::parse(input, language).map_err(|e| format!("Parse error: {:?}", e))?;

    ChronoProvider.parse_expression(expr)
}
