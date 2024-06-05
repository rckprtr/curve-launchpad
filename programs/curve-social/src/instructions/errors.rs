use anchor_lang::error_code;


#[error_code]
pub enum CurveSocialError {
    #[msg("Global Already Initialized")]
    AlreadyInitialized,
}