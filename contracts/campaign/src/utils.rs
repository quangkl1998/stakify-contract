use cosmwasm_std::{Uint128, OverflowError, DivideByZeroError};


/// Calculates the reward amount
pub fn add_reward(current_reward: Uint128, calc_reward: Uint128) -> Result<Uint128, OverflowError> {
    current_reward
        .checked_add(calc_reward)
}

pub fn sub_reward(current_reward: Uint128, calc_reward: Uint128) -> Result<Uint128, OverflowError> {
    current_reward
        .checked_sub(calc_reward)
}

pub fn calc_reward_in_time(
    start_time: u64,
    end_time: u64,
    reward_per_second: Uint128,
    percent: Uint128,
    nft_count: u128,
) -> Result<Uint128, DivideByZeroError> {
    let diff_time = end_time.checked_sub(start_time).unwrap_or(0);

    let mul_reward = Uint128::from(diff_time)
        .checked_mul(reward_per_second)
        .and_then(|res| res.checked_mul(percent))
        .unwrap();

    let divisor = Uint128::from(100 as u128)
        .checked_mul(Uint128::from(nft_count))
        .unwrap();

    let final_reward = mul_reward.checked_div(divisor);

    final_reward
}
