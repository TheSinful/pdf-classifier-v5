use crate::context::ContextUpdate;
use crate::generated::generated_object_types::ObjectCastError;
use crate::score::Score;
use crate::weighting::constraints::{HARD_ENUM_VARIANT_COUNT, HardConstraints};
use crate::{
    context::{Context, ContextUpdateHistory},
    generated::generated_object_types::{KnownObject, OBJECT_COUNT},
    page::Page,
    weighting::constraints::{SOFT_ENUM_VARIANT_COUNT, SoftConstraints},
};
use eq_float::F32 as HashableF32;
use std::{collections::HashMap, rc::Rc};

mod constraints;

#[derive(Hash)]
pub struct Weight {
    value: HashableF32,
    constraint: SoftConstraints,
    changes: Vec<String>,
}

impl Weight {
    pub fn new(for_constraint: SoftConstraints) -> Self {
        Self {
            value: 0.0.into(),
            constraint: for_constraint,
            changes: Vec::new(),
        }
    }
}

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
}

type ScoreManagerResult<T> = std::result::Result<T, ScoreManagerError>;
type WeightList = Vec<Weight>;

struct ConstraintList(HashMap<KnownObject, WeightList>);

impl ConstraintList {
    pub fn new() -> ScoreManagerResult<Self> {
        let mut coll = HashMap::new();

        for class_discrim in 0..OBJECT_COUNT {
            let class: KnownObject = class_discrim.try_into()?;

            let mut weights = Vec::new();

            for weight_type in 0..SOFT_ENUM_VARIANT_COUNT {
                let constraint = weight_type.try_into()?;

                weights.push(Weight::new(constraint));
            }

            coll.insert(class, weights);
        }

        Ok(Self { 0: coll })
    }

    pub fn weights(&mut self, class: &KnownObject) -> ScoreManagerResult<&mut WeightList> {
        match self.0.get_mut(class) {
            Some(s) => Ok(s),
            None => Err(ScoreManagerError::FailedToLocateWeights(class.to_string())),
        }
    }
}

pub struct InferenceResult {
    candidates: Vec<KnownObject>,
}

struct KnownObjectList(pub Vec<KnownObject>);

impl KnownObjectList {
    pub fn new(v: Vec<KnownObject>) -> Self {
        Self { 0: v }
    }

    pub fn filter_by_hard_constraints(
        &self,
        ctx: &Context,
        page: Page,
    ) -> ScoreManagerResult<Self> {
        let mut result = Vec::new();

        for hard_constraint in 0..HARD_ENUM_VARIANT_COUNT {
            let constraint: HardConstraints = hard_constraint.try_into()?;

            result = self
                .0
                .iter()
                .filter(|x| constraint.eval(ctx, **x, page))
                .cloned()
                .collect::<Vec<KnownObject>>();
        }

        Ok(Self { 0: result })
    }

    pub fn sort_by_soft_constraints(
        &mut self,
        ctx: &Context,
        page: Page,
    ) -> ScoreManagerResult<()> {
        fn eval_class(
            ctx: &Context,
            class: KnownObject,
            page: Page,
            constraint: SoftConstraints,
            collection: &mut Vec<(KnownObject, Vec<Score>)>,
        ) -> ScoreManagerResult<()> {
            let score = constraint.eval(ctx, class, page);
            let position = collection.iter().position(|x| x.0 == class).ok_or({
                ScoreManagerError::ScoreMapMissingConstraint(class.to_string(), constraint)
            })?;

            collection[position].1.push(score);

            Ok(())
        }

        let mut scores: Vec<(KnownObject, Vec<Score>)> = Vec::new();
        for soft_constraint_idx in 0..SOFT_ENUM_VARIANT_COUNT {
            let soft_constraint: SoftConstraints = soft_constraint_idx.try_into()?;

            self.0
                .iter()
                .try_for_each(|x| eval_class(ctx, *x, page, soft_constraint, &mut scores))?;
        }

        todo!()
    }
}

pub struct ScoreManager {
    weighted_constraints: ConstraintList,
    ctx: Rc<Context>,
    all_candidates: KnownObjectList,
}

impl ScoreManager {
    pub fn new(ctx: Rc<Context>) -> ScoreManagerResult<Self> {
        let all_candidates = {
            let mut result = Vec::new();

            for class in 0..OBJECT_COUNT {
                let class: KnownObject = class.try_into()?;

                result.push(class);
            }

            result
        };

        Ok(Self {
            weighted_constraints: ConstraintList::new()?,
            ctx,
            all_candidates: KnownObjectList::new(all_candidates),
        })
    }

    pub fn infer(&self, ctx: &Context, page: Page) -> ScoreManagerResult<InferenceResult> {
        let valid_candidates = self.all_candidates.filter_by_hard_constraints(ctx, page)?;

        todo!()
    }

    pub fn step(&mut self, history: &mut ContextUpdateHistory) -> ScoreManagerResult<()> {
        // history provides us the classification/inference knowledge between each step
        // so, we can now mutate our weights based off changes in said history

        for update in history {
            match update {
                ContextUpdate::Decision(page, class) => self.handle_decision(page, class)?,
                ContextUpdate::NewParent(_) => todo!(),
            };
        }

        Ok(())
    }

    fn handle_decision(&mut self, page: &Page, for_class: &KnownObject) -> ScoreManagerResult<()> {
        let weights = self.weighted_constraints.weights(for_class)?;

        Ok(())
    }
}
