use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::generated::generated_object_types::OBJECT_COUNT;
use crate::inferencer::{InferenceError, InferenceResult};
use crate::page::Page;
use crate::score::Score;
use crate::weighting::constraints::DefinitiveConstraints;
use crate::weighting::constraints::HardConstraints;
use crate::weighting::constraints::{
    DEFINITIVE_ENUM_VARIANT_COUNT, HARD_ENUM_VARIANT_COUNT, SOFT_ENUM_VARIANT_COUNT,
    SoftConstraints,
};

pub(crate) struct KnownObjectList(pub Vec<KnownObject>);

impl KnownObjectList {
    pub fn new() -> InferenceResult<Self> {
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
    ) -> InferenceResult<Self> {
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

    pub fn filter_by_hard_constraints(self, ctx: &Context, page: Page) -> InferenceResult<Self> {
        let before = self.0.len();
        let mut result = self.0;

        for hard_constraint in 0..HARD_ENUM_VARIANT_COUNT {
            let constraint: HardConstraints = hard_constraint.try_into()?;

            result = result
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

    pub fn sort_by_soft_constraints(self, ctx: &Context, page: Page) -> InferenceResult<Self> {
        pub(crate) fn eval_class(
            ctx: &Context,
            class: KnownObject,
            page: Page,
            constraint: SoftConstraints,
            scores: &mut Vec<(KnownObject, Vec<Score>)>,
        ) -> InferenceResult<()> {
            let score = constraint.eval(ctx, class, page);
            log::trace!(
                "page {} class {} scored {:?} on soft constraint {:?}",
                page,
                class.to_string(),
                score,
                constraint
            );

            let position = scores.iter().position(|x| x.0 == class).ok_or({
                InferenceError::ScoreMapMissingConstraint(class.to_string(), constraint)
            })?;

            scores[position].1.push(score);
            Ok(())
        }

        // Build a score entry for every candidate in this list (not all objects).
        let mut scores: Vec<(KnownObject, Vec<Score>)> = self
            .0
            .iter()
            .map(|obj| (*obj, Vec::with_capacity(SOFT_ENUM_VARIANT_COUNT as usize)))
            .collect();

        for soft_constraint_idx in 0..SOFT_ENUM_VARIANT_COUNT {
            let soft_constraint: SoftConstraints = soft_constraint_idx.try_into()?;

            self.0
                .iter()
                .try_for_each(|x| eval_class(ctx, *x, page, soft_constraint, &mut scores))?;
        }

        scores.iter_mut().for_each(|x| x.1.sort_by(|a, b| a.cmp(b)));
        scores.sort_by(|x, y| {
            x.1.last()
                .copied()
                .unwrap_or_default()
                .cmp(&y.1.last().copied().unwrap_or_default())
        });

        if let Some(top) = scores.last() {
            log::trace!(
                "page {} soft sort complete, top candidate is {}",
                page,
                top.0.to_string()
            );
        }

        let sorted: Vec<KnownObject> = scores.into_iter().map(|(obj, _)| obj).collect();
        Ok(Self { 0: sorted })
    }
}
