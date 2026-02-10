use cosmwasm_std::{Uint128, StdResult};
use crate::msg::VestingSchedule;

pub fn calculate_vested_amount(
    schedule: &VestingSchedule,
    total_amount: Uint128,
    current_time: u64,
) -> StdResult<Uint128> {
    match schedule {
        VestingSchedule::Linear {
            start_time,
            end_time,
            cliff_time,
            release_interval,
        } => {
            if let Some(cliff) = cliff_time {
                if current_time < *cliff {
                    return Ok(Uint128::zero());
                }
            } else if current_time < *start_time {
                return Ok(Uint128::zero());
            }

            if current_time >= *end_time {
                return Ok(total_amount);
            }

            let duration = end_time - start_time;
            if duration == 0 {
                return Ok(total_amount);
            }

            let elapsed = current_time - start_time;
            let interval = *release_interval;
            let effective_elapsed = if interval <= 1 {
                elapsed
            } else {
                (elapsed / interval) * interval
            };

            // vested = total_amount * effective_elapsed / duration
            // Use u128 for calculation to avoid overflow
            let vested = total_amount.multiply_ratio(effective_elapsed, duration);
            Ok(vested)
        }
        VestingSchedule::Custom { milestones } => {
            let mut vested = Uint128::zero();
            for milestone in milestones {
                if current_time >= milestone.timestamp {
                    vested += milestone.amount;
                }
            }
            Ok(vested)
        }
    }
}

pub fn validate_schedule(schedule: &VestingSchedule, total_amount: Uint128) -> Result<(), String> {
    match schedule {
        VestingSchedule::Linear {
            start_time,
            end_time,
            cliff_time,
            release_interval,
        } => {
            if end_time < start_time {
                return Err("end_time must be >= start_time".to_string());
            }
            if let Some(cliff) = cliff_time {
                if *cliff < *start_time {
                    return Err("cliff_time must be >= start_time".to_string());
                }
                if *cliff > *end_time {
                    return Err("cliff_time must be <= end_time".to_string());
                }
            }
            if *release_interval == 0 {
                return Err("release_interval must be > 0".to_string());
            }
        }
        VestingSchedule::Custom { milestones } => {
            if milestones.is_empty() {
                return Err("milestones cannot be empty".to_string());
            }
            let mut sum = Uint128::zero();
            let mut last_timestamp = 0;
            for milestone in milestones {
                if milestone.timestamp < last_timestamp {
                    return Err("milestones must be sorted by timestamp".to_string());
                }
                last_timestamp = milestone.timestamp;
                sum += milestone.amount;
            }
            if sum != total_amount {
                return Err(format!(
                    "sum of milestone amounts ({}) must equal total vested amount ({})",
                    sum, total_amount
                ));
            }
        }
    }
    Ok(())
}
