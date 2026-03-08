use crate::generated::generated_object_types::ObjectCastError;
use crate::score::Score;
use crate::weighting::constraints::{
    DEFINITIVE_ENUM_VARIANT_COUNT, DefinitiveConstraints, HARD_ENUM_VARIANT_COUNT, HardConstraints,
};
use crate::{
    context::{Context, ContextUpdateHistory},
    generated::generated_object_types::{KnownObject, OBJECT_COUNT},
    page::Page,
    weighting::constraints::{SOFT_ENUM_VARIANT_COUNT, SoftConstraints},
};

mod constraints;

#[derive(thiserror::Error, Debug)]
pub enum ScoreManagerError {
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

type ScoreManagerResult<T> = std::result::Result<T, ScoreManagerError>;

pub struct InferenceResult {
    candidates: Vec<KnownObject>,
}

struct KnownObjectList(pub Vec<KnownObject>);

impl KnownObjectList {
    pub fn new() -> ScoreManagerResult<Self> {
        let mut vec: Vec<KnownObject> = vec![];

        for discrim in 0..OBJECT_COUNT {
            let obj = KnownObject::try_from(discrim)?;
            vec.push(obj);
        }

        log::trace!("built candidate list with {} objects", vec.len());

        Ok(Self { 0: vec })
    }

    pub fn filter_by_definitive_constraints(
        self,
        ctx: &Context,
        page: Page,
    ) -> ScoreManagerResult<Self> {
        let mut result = Vec::with_capacity(1);

        for def_constraint_discrim in 0..DEFINITIVE_ENUM_VARIANT_COUNT {
            let def_constraint: DefinitiveConstraints = def_constraint_discrim.try_into()?;
            let found = self.0.iter().find(|x| def_constraint.eval(ctx, **x, page));

            match found {
                Some(class) => {
                    log::trace!(
                        "page {} hit definitive constraint {:?}, winner is {}",
                        page,
                        def_constraint,
                        class.to_string()
                    );
                    result.push(*class);
                    return Ok(Self { 0: result });
                }
                None => {
                    log::trace!(
                        "page {} no match on definitive constraint {:?}",
                        page,
                        def_constraint
                    );
                    continue;
                }
            }
        }

        log::trace!(
            "page {} passed all definitive constraints with no match, returning full candidate list",
            page
        );
        Ok(Self { 0: self.0 })
    }

    pub fn filter_by_hard_constraints(self, ctx: &Context, page: Page) -> ScoreManagerResult<Self> {
        let mut result = Vec::new();
        let before = self.0.len();

        for hard_constraint in 0..HARD_ENUM_VARIANT_COUNT {
            let constraint: HardConstraints = hard_constraint.try_into()?;

            result = self
                .0
                .iter()
                .filter(|x| constraint.eval(ctx, **x, page))
                .cloned()
                .collect::<Vec<KnownObject>>();

            log::trace!(
                "page {} after hard constraint {:?}: {} candidates remaining",
                page,
                constraint,
                result.len()
            );
        }

        log::trace!(
            "page {} hard filtering done, {} -> {} candidates",
            page,
            before,
            result.len()
        );
        Ok(Self { 0: result })
    }

    pub fn sort_by_soft_constraints(self, ctx: &Context, page: Page) -> ScoreManagerResult<Self> {
        fn eval_class(
            ctx: &Context,
            class: KnownObject,
            page: Page,
            constraint: SoftConstraints,
            scores: &mut Vec<(KnownObject, Vec<Score>)>,
        ) -> ScoreManagerResult<()> {
            let score = constraint.eval(ctx, class, page);
            log::trace!(
                "page {} class {} scored {:?} on soft constraint {:?}",
                page,
                class.to_string(),
                score,
                constraint
            );

            let position = scores.iter().position(|x| x.0 == class).ok_or({
                ScoreManagerError::ScoreMapMissingConstraint(class.to_string(), constraint)
            })?;

            scores[position].1.push(score);
            Ok(())
        }

        let mut scores: Vec<(KnownObject, Vec<Score>)> = Vec::with_capacity(OBJECT_COUNT as usize);
        for i in 0..OBJECT_COUNT {
            scores[i as usize].0 = i.try_into()?;
            scores[i as usize].1 = Vec::with_capacity(SOFT_ENUM_VARIANT_COUNT as usize);
        }

        for soft_constraint_idx in 0..SOFT_ENUM_VARIANT_COUNT {
            let soft_constraint: SoftConstraints = soft_constraint_idx.try_into()?;

            self.0
                .iter()
                .try_for_each(|x| eval_class(ctx, *x, page, soft_constraint, &mut scores))?;
        }

        scores.iter_mut().for_each(|x| x.1.sort_by(|x, y| x.cmp(y)));
        scores.sort_by(|x, y| x.1.last().unwrap().cmp(y.1.last().unwrap()));

        log::trace!(
            "page {} soft sort complete, top candidate is {}",
            page,
            scores.last().unwrap().0.to_string()
        );

        Ok(self)
    }
}

pub struct ScoreManager;

impl ScoreManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn step(
        &mut self,
        history: &mut ContextUpdateHistory,
        ctx: &mut Context,
        pages: Vec<Page>,
    ) -> ScoreManagerResult<()> {
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
                ctx.decide(page, winner, history)?;
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
            log::trace!("page {} final inference: {}", page, winner.to_string());
            ctx.decide(page, winner, history)?
        }

        Ok(())
    }

    // fn handle_decision(&mut self, page: &Page, for_class: &KnownObject) -> ScoreManagerResult<()> {
    //     Ok(())
    // }
}

#[cfg(test)]
mod tests {
    use crate::{
        context::{Context, ContextUpdateHistory},
        weighting::ScoreManager,
    };

    #[test]
    pub fn test_score_manage_step() {
        let mut manager = ScoreManager::new();
        let mut mock_history = ContextUpdateHistory::new();
        let mut ctx = Context::new(10, 0u32.into(), 9u32.into());
        let mut pages = Vec::new();
        pages.push(0u32.into());

        match manager.step(&mut mock_history, &mut ctx, pages) {
            Ok(_) => {}
            Err(e) => panic!("{}", e),
        }
    }
}
