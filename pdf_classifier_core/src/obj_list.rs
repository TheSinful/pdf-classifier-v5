use crate::constraints::DefinitiveConstraints;
use crate::constraints::HardConstraints;
use crate::constraints::{
    DEFINITIVE_ENUM_VARIANT_COUNT, HARD_ENUM_VARIANT_COUNT, SOFT_ENUM_VARIANT_COUNT,
    SoftConstraints,
};
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::generated::generated_object_types::OBJECT_COUNT;
use crate::inferencer::{InferenceError, InferenceResult};
use crate::page::Page;
use crate::score::Score;
use std::collections::HashSet;
use std::ops::Deref;
use std::ops::DerefMut;

#[derive(Debug)]
struct ScoreList(pub Vec<Score>);

impl ScoreList {
    pub fn sum(&self) -> Score {
        self.0.iter().sum()
    }
}

impl Deref for ScoreList {
    type Target = Vec<Score>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ScoreList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone)]
pub struct KnownObjectList(pub Vec<KnownObject>);

impl KnownObjectList {
    pub fn new() -> Self {
        let mut vec: Vec<KnownObject> = vec![];

        for discrim in 0..OBJECT_COUNT {
            let obj = KnownObject::try_from(discrim).unwrap();
            vec.push(obj);
        }

        log::trace!("built candidate list with {} objects", vec.len());

        Self { 0: vec }
    }

    pub fn from_vec(vec: Vec<KnownObject>) -> Self {
        log::trace!("built candidate list with {} objects", vec.len());

        Self { 0: vec }
    }

    pub fn filter_out(self, guaranteed_not_to_be: &Vec<KnownObject>) -> Self {
        let mut set: HashSet<_> = self.0.into_iter().collect();
        set.retain(|x| !guaranteed_not_to_be.contains(x));

        Self {
            0: set.into_iter().collect(),
        }
    }

    fn filter_out_failed(mut self, ctx: &Context, page: &Page) -> Self {
        if let Some(results) = ctx.guarantee_failures.0.get(page) {
            self = self.filter_out(results);
        }

        self
    }

    pub fn filter_by_definitive_constraints(
        mut self,
        ctx: &Context,
        page: Page,
    ) -> InferenceResult<Self> {
        // ! similar issue to [KnownObjectList::filter_by_hard_constraints], see comment block there.

        let mut result = Vec::new();
        self = self.filter_out_failed(ctx, &page);

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

        Ok(self)
    }

    pub fn filter_by_hard_constraints(
        mut self,
        ctx: &Context,
        page: Page,
    ) -> InferenceResult<Self> {
        /*
            ! this implementation may be slightly flawed
            ! since we inherently iterate over the unknown class variant (discrim=0)
            ! we will validate it against constraints, and inevitably it will fail
            ! therefore, this will be zero and later on the pipe-line we reconstruct following. unwrap_or_default()
            ! as unknown, which is exactly what we just discarded.
            ! therefore, this can be optimized by never touching unknown.
            ! or maybe not, since .unwrap() will be just as time-consuming as .unwrap_or_default(), the only save
            ! really being with .unwrap_unchecked()? but i'm not sure if the safety trade-off is worth it here.
            ! maybe if this holds stability and remains constant in terms of changes that trade-off could be made.
        */

        let before_count = self.0.len();
        self = self.filter_out_failed(ctx, &page);

        for hard_constraint in 0..HARD_ENUM_VARIANT_COUNT {
            let constraint: HardConstraints = hard_constraint.try_into()?;

            self.0 = self
                .0
                .iter()
                .filter(|x| constraint.eval(ctx, **x, page))
                .cloned()
                .collect::<Vec<KnownObject>>();

            log::trace!(
                "page {} after hard constraint {:?}: {} candidates remaining ({:?})",
                page,
                constraint,
                self.0.len(),
                self.0
            );
        }

        log::trace!(
            "page {} hard filtering done, {} -> {} candidates",
            page,
            before_count,
            self.0.len()
        );

        Ok(self)
    }

    pub fn sort_by_soft_constraints(mut self, ctx: &Context, page: Page) -> InferenceResult<Self> {
        self = self.filter_out_failed(ctx, &page);

        fn eval_class(
            ctx: &Context,
            class: KnownObject,
            page: Page,
            constraint: SoftConstraints,
            scores: &mut Vec<(KnownObject, ScoreList)>,
        ) -> InferenceResult<()> {
            let score = constraint.eval(ctx, class, page);
            log::trace!(
                "page {}, class {} scored: {:?} on soft constraint: {:?}",
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

        if self.0.len() <= 1 {
            return Ok(self);
        }

        let mut scores: Vec<(KnownObject, ScoreList)> = self
            .0
            .iter() // skip unknown variant
            .map(|i| {
                Ok((
                    *i,
                    ScoreList {
                        0: Vec::with_capacity(SOFT_ENUM_VARIANT_COUNT as usize),
                    },
                ))
            })
            .collect::<InferenceResult<_>>()?;

        for soft_constraint_idx in 0..SOFT_ENUM_VARIANT_COUNT {
            let soft_constraint: SoftConstraints = soft_constraint_idx.try_into()?;

            self.0
                .iter()
                .try_for_each(|x| eval_class(ctx, *x, page, soft_constraint, &mut scores))?;
        }

        scores.iter_mut().for_each(|x| x.1.sort_by(|x, y| x.cmp(y)));

        scores.sort_by(|x, y| x.1.sum().cmp(&y.1.sum()));

        log::trace!(
            "page {} soft sort complete, top candidate is {}, (all candidates) {:?} ",
            page,
            scores.last().unwrap().0.to_string(),
            scores
        );

        Ok(Self {
            0: scores.into_iter().map(|f| f.0).collect(),
        })
    }
}
