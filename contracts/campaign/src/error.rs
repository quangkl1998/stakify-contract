use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("##Unauthorized##")]
    Unauthorized {},

    #[error("## You are not the owner of this NFT ##")]
    NotOwner { token_id: String },

    #[error("## You have reached the maximum staked NFTs ##")]
    LimitPerStake {},

    #[error("## Invalid funds ##")]
    InvalidFunds {},

    #[error("## Max 3 years since start date ##")]
    LimitStartDate {},

    #[error("## Max limit {max:?} character ##")]
    LimitCharacter { max: String },

    #[error("## Invalid Token ##")]
    InvalidToken {},

    #[error("## Invalid LockupTerm ##")]
    InvalidLockupTerm {},

    #[error("## Insufficient balance ##")]
    InsufficientBalance {},

    #[error("## Too many token ids ##")]
    TooManyTokenIds {},

    #[error("## Invalid time to update ##")]
    InvalidTimeToUpdate {},

    #[error("## This campaign is not available for staking ##")]
    InvalidTimeToStakeNft {},

    #[error("## This NFT is still in staking period. Cannot unstake now ##")]
    InvalidTimeToUnStake {},

    #[error("## Cannot deposit rewards to this pool ##")]
    InvalidTimeToAddReward {},

    #[error("## Reward has been added##")]
    RewardAdded {},

    #[error("## Only stakers could claim rewards in this pool ##")]
    InvalidClaim {},

    #[error("## Invalid time to withdraw reward ##")]
    InvalidTimeToWithdrawReward {},

    #[error("## Already exist ##")]
    AlreadyExist {},

    #[error("## Empty reward pool ##")]
    EmptyReward {},

    #[error("## Empty token_id: {token_id:?} ##")]
    EmptyNft { token_id: String},
}
