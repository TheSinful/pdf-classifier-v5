use tracing::instrument;

use crate::constraints;
use crate::generated::generated_object_types::ObjectCastError;
use crate::obj_list::KnownObjectList;
use crate::{
    constraints::SoftConstraints, context::Context, generated::generated_object_types::KnownObject,
    page::Page,
};

#[derive(thiserror::Error, Debug)]
pub enum InferenceError {
    #[error(transparent)]
    ClassCastError(#[from] ObjectCastError),

    #[error(transparent)]
    ConstraintCastError(#[from] constraints::CastError),

    #[error(
        "Attempted to score on class {0} with SoftConstriant {1}, yet no constraint was stored in score map."
    )]
    ScoreMapMissingConstraint(String, SoftConstraints),

    #[error(transparent)]
    ContextError(#[from] crate::context::ContextError),
}

pub type InferenceResult<T> = std::result::Result<T, InferenceError>;

#[derive(Clone, Copy, Debug)]
pub struct Inferencer;

impl Inferencer {
    pub fn new() -> Self {
        Self {}
    }

    #[instrument(skip_all, fields(%page, candidates = ?candidates.0), ret, err)]
    pub fn infer(
        &mut self,
        ctx: &mut Context,
        page: Page,
        candidates: &KnownObjectList,
    ) -> InferenceResult<KnownObject> {
        let candidates_after_def_constraint = candidates
            .clone()
            .filter_by_definitive_constraints(ctx, page)?;

        if candidates_after_def_constraint.len() == 1 {
            let winner = *candidates_after_def_constraint.0.last().unwrap();

            return Ok(winner);
        }

        let candidates_after_soft_hard_constraints = candidates_after_def_constraint
            .filter_by_hard_constraints(ctx, page)?
            .sort_by_soft_constraints(ctx, page)?;

        // todo: apply dynamic weighting here or maybe take an arg in "sort" to do so

        let winner = *candidates_after_soft_hard_constraints
            .0
            .last()
            .unwrap_or_else(|| &KnownObject::UNKNOWN);

        return Ok(winner);
    }
}

#[cfg(test)]
mod tests {
    use super::Inferencer;
    use crate::{
        context::Context, generated::generated_object_types::KnownObject,
        obj_list::KnownObjectList, page::Page,
    };

    #[test]
    pub fn test_score_manage_step() {
        let start_page = Page(0);
        let end_page = Page(9);

        let mut manager = Inferencer::new();
        let mut ctx = Context::new(start_page, end_page);

        match manager.infer(&mut ctx, start_page, &KnownObjectList::new()) {
            Ok(r) => {
                assert!(r == KnownObject::CHAPTER); // TODO! need something to force the example project for cfg(test) otherwise this will fail compilation.
            }
            Err(e) => panic!("{}", e),
        }
    }
}
