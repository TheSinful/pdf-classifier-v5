use crate::generated::generated_object_types::ObjectCastError;
use crate::obj_list::KnownObjectList;
use crate::weighting::constraints;
use crate::{
    context::Context, generated::generated_object_types::KnownObject, page::Page,
    weighting::constraints::SoftConstraints,
};

#[derive(thiserror::Error, Debug)]
pub enum InferenceError {
    #[error(transparent)]
    ClassCastError(#[from] ObjectCastError),

    #[error(transparent)]
    ConstraintCastError(#[from] constraints::CastError),

    #[error("Attempted to access an out of bounds SoftConstraint, Given i={0} when {1} >= i >= 0")]
    SoftConstraintOutOfBounds(u8, u8), // given, max

    #[error(
        "Failed to find stored weights of class {0}! This shouldn't ever happen as at the minimum each class gets default weights!"
    )]
    FailedToLocateWeights(String),

    #[error(
        "Attempted to score on class {0} with SoftConstriant {1}, yet no constraint was stored in score map."
    )]
    ScoreMapMissingConstraint(String, SoftConstraints),

    #[error(transparent)]
    ContextError(#[from] crate::context::ContextError),
}

pub type InferenceResult<T> = std::result::Result<T, InferenceError>;

pub struct Inferencer;

impl Inferencer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn infer(
        &mut self,
        ctx: &mut Context,
        pages: Vec<Page>,
    ) -> InferenceResult<Vec<KnownObject>> {
        let mut winners: Vec<KnownObject> = vec![];
        let total_pages = pages.len();

        for page in pages {
            log::trace!("beginning inference on page {}", page);

            let candidates = KnownObjectList::new()?.filter_by_definitive_constraints(ctx, page)?;

            if candidates.0.len() == 1 {
                let winner = *candidates.0.last().unwrap();
                log::trace!(
                    "page {} resolved early via definitive constraint as {}",
                    page,
                    winner.to_string()
                );

                winners.push(winner);
                continue;
            }

            log::trace!(
                "page {} proceeding to hard + soft filtering with {} candidates",
                page,
                candidates.0.len()
            );

            let candidates = candidates
                .filter_by_hard_constraints(ctx, page)?
                .sort_by_soft_constraints(ctx, page)?;

            // todo: apply dynamic weighting here or maybe take an arg in "sort" to do so

            let winner = *candidates.0.last().unwrap();
            winners.push(winner);
            log::trace!("page {} final inference: {}", page, winner.to_string());
        }

        log::trace!(
            "done stepping over {} page(s), with {} winner(s)",
            total_pages,
            winners.len()
        );

        Ok(winners)
    }
}

#[cfg(test)]
mod tests {
    use super::Inferencer;
    use crate::{context::Context, generated::generated_object_types::KnownObject};

    #[test]
    pub fn test_score_manage_step() {
        let mut manager = Inferencer::new();
        let mut ctx = Context::new(0u32.into(), 9u32.into());
        let mut pages = Vec::new();
        pages.push(0u32.into());

        match manager.infer(&mut ctx, pages) {
            Ok(r) => {
                assert!(
                    r.len() == 1,
                    "Should've gotten one winner but got {}",
                    r.len()
                );
                assert!(*r.last().unwrap() == KnownObject::CHAPTER); // TODO! need something to force the example project for cfg(test) otherwise this will fail compilation.
            }
            Err(e) => panic!("{}", e),
        }
    }
}
