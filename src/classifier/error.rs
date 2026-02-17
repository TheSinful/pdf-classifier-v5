#[derive(thiserror::Error, Debug)]
pub enum ClassifierError {
    #[error("Attempted to end deference before a viable page was found to end it.")]
    AttemptedToEndDeferenceEarly,
}

