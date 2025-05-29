use jiff::{Span, Zoned};
use temps_core::{
    DayReference, Direction, Language, TimeExpression, TimeParser, TimeUnit, Weekday,
    errors::*,
    time_utils::{
        calculate_timezone_offset_seconds, calculate_weekday_offset, convert_12_to_24_hour,
    },
};

pub struct JiffProvider;

impl TimeParser for JiffProvider {
    type DateTime = Zoned;
    type Error = String;

    fn now(&self) -> Self::DateTime {
        Zoned::now()
    }

    fn parse_expression(&self, expr: TimeExpression) -> Result<Self::DateTime, Self::Error> {
        match expr {
            TimeExpression::Now => Ok(self.now()),
            TimeExpression::Relative(rel) => {
                let now = self.now();

                // Create a span based on the time unit
                let span = match rel.unit {
                    TimeUnit::Second => Span::new().seconds(rel.amount),
                    TimeUnit::Minute => Span::new().minutes(rel.amount),
                    TimeUnit::Hour => Span::new().hours(rel.amount),
                    TimeUnit::Day => Span::new().days(rel.amount),
                    TimeUnit::Week => Span::new().weeks(rel.amount),
                    TimeUnit::Month => Span::new().months(rel.amount),
                    TimeUnit::Year => Span::new().years(rel.amount),
                };

                // Apply the span in the correct direction
                match rel.direction {
                    Direction::Past => now
                        .checked_sub(span)
                        .map_err(|e| format!("{}: {}", ERR_DATE_CALC_ERROR, e)),
                    Direction::Future => now
                        .checked_add(span)
                        .map_err(|e| format!("{}: {}", ERR_DATE_CALC_ERROR, e)),
                }
            }
            TimeExpression::Absolute(abs) => {
                use jiff::civil::{Date, DateTime, Time};
                use jiff::tz::{Offset, TimeZone};

                let date = Date::new(abs.year as i16, abs.month as i8, abs.day as i8)
                    .map_err(|e| format!("{}: {}", ERR_INVALID_DATE, e))?;

                if let (Some(hour), Some(minute)) = (abs.hour, abs.minute) {
                    let time = Time::new(
                        hour as i8,
                        minute as i8,
                        abs.second.unwrap_or(0) as i8,
                        abs.nanosecond.unwrap_or(0) as i32,
                    )
                    .map_err(|e| format!("{}: {}", ERR_INVALID_TIME, e))?;

                    let datetime = DateTime::from_parts(date, time);

                    match &abs.timezone {
                        Some(temps_core::Timezone::Utc) => datetime
                            .to_zoned(TimeZone::UTC)
                            .map(|z| z.with_time_zone(TimeZone::system()))
                            .map_err(|e| format!("Timezone conversion error: {}", e)),
                        Some(temps_core::Timezone::Offset { hours, minutes }) => {
                            let total_seconds = calculate_timezone_offset_seconds(*hours, *minutes);
                            let offset = Offset::from_seconds(total_seconds)
                                .map_err(|e| format!("Invalid offset: {}", e))?;

                            datetime
                                .to_zoned(TimeZone::fixed(offset))
                                .map(|z| z.with_time_zone(TimeZone::system()))
                                .map_err(|e| format!("Timezone conversion error: {}", e))
                        }
                        None => {
                            // No timezone specified, treat as system timezone
                            datetime
                                .to_zoned(TimeZone::system())
                                .map_err(|e| format!("Timezone conversion error: {}", e))
                        }
                    }
                } else {
                    // Date only, set time to midnight
                    let datetime = date.at(0, 0, 0, 0);
                    datetime
                        .to_zoned(TimeZone::system())
                        .map_err(|e| format!("Timezone conversion error: {}", e))
                }
            }
            TimeExpression::Day(day_ref) => {
                let now = self.now();
                match day_ref {
                    DayReference::Today => {
                        let date = now.date();
                        date.at(0, 0, 0, 0)
                            .to_zoned(now.time_zone().clone())
                            .map_err(|e| format!("Failed to create today's date: {}", e))
                    }
                    DayReference::Yesterday => {
                        let yesterday = now
                            .checked_sub(Span::new().days(1))
                            .map_err(|e| format!("Failed to calculate yesterday: {}", e))?;
                        let date = yesterday.date();
                        date.at(0, 0, 0, 0)
                            .to_zoned(now.time_zone().clone())
                            .map_err(|e| format!("Failed to create yesterday's date: {}", e))
                    }
                    DayReference::Tomorrow => {
                        let tomorrow = now
                            .checked_add(Span::new().days(1))
                            .map_err(|e| format!("Failed to calculate tomorrow: {}", e))?;
                        let date = tomorrow.date();
                        date.at(0, 0, 0, 0)
                            .to_zoned(now.time_zone().clone())
                            .map_err(|e| format!("Failed to create tomorrow's date: {}", e))
                    }
                    DayReference::Weekday { day, modifier } => {
                        let target_weekday = match day {
                            Weekday::Monday => jiff::civil::Weekday::Monday,
                            Weekday::Tuesday => jiff::civil::Weekday::Tuesday,
                            Weekday::Wednesday => jiff::civil::Weekday::Wednesday,
                            Weekday::Thursday => jiff::civil::Weekday::Thursday,
                            Weekday::Friday => jiff::civil::Weekday::Friday,
                            Weekday::Saturday => jiff::civil::Weekday::Saturday,
                            Weekday::Sunday => jiff::civil::Weekday::Sunday,
                        };

                        let current_weekday = now.weekday();
                        let current_offset = current_weekday.to_monday_zero_offset() as i64;
                        let target_offset = target_weekday.to_monday_zero_offset() as i64;

                        let days_to_add =
                            calculate_weekday_offset(current_offset, target_offset, modifier);
                        let target_date = now.checked_add(Span::new().days(days_to_add));

                        let target = target_date
                            .map_err(|e| format!("Failed to calculate weekday: {}", e))?;
                        let date = target.date();
                        date.at(0, 0, 0, 0)
                            .to_zoned(now.time_zone().clone())
                            .map_err(|e| format!("Failed to create weekday date: {}", e))
                    }
                }
            }
            TimeExpression::Time(time) => {
                let now = self.now();
                let date = now.date();
                let hour = convert_12_to_24_hour(time.hour, time.meridiem.as_ref()) as i8;

                date.at(hour, time.minute as i8, time.second as i8, 0)
                    .to_zoned(now.time_zone().clone())
                    .map_err(|e| format!("Failed to create time: {}", e))
            }
            TimeExpression::DayTime(day_time) => {
                // First get the day
                let day_result =
                    self.parse_expression(TimeExpression::Day(day_time.day.clone()))?;
                let date = day_result.date();

                let hour =
                    convert_12_to_24_hour(day_time.time.hour, day_time.time.meridiem.as_ref())
                        as i8;

                date.at(
                    hour,
                    day_time.time.minute as i8,
                    day_time.time.second as i8,
                    0,
                )
                .to_zoned(day_result.time_zone().clone())
                .map_err(|e| format!("Failed to create day time: {}", e))
            }
            TimeExpression::Date(date) => {
                use jiff::civil::Date;

                let jiff_date = Date::new(date.year as i16, date.month as i8, date.day as i8)
                    .map_err(|e| format!("{}: {}", ERR_INVALID_DATE, e))?;

                jiff_date
                    .at(0, 0, 0, 0)
                    .to_zoned(jiff::tz::TimeZone::system())
                    .map_err(|e| format!("Failed to create date: {}", e))
            }
        }
    }
}

pub fn parse_to_zoned(input: &str, language: Language) -> Result<Zoned, String> {
    let expr = temps_core::parse(input, language).map_err(|e| format!("Parse error: {:?}", e))?;

    JiffProvider.parse_expression(expr)
}
