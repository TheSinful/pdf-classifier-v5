use crate::context::ContextUpdate;
use crate::generated::generated_object_types::ObjectCastError;
use crate::{
    context::{Context, ContextUpdateHistory},
    generated::generated_object_types::{KnownObject, OBJECT_COUNT},
    page::Page,
    weighting::constraints::{SOFT_ENUM_VARIANT_COUNT, SoftConstraints},
};
use eq_float::F32 as HashableF32;
use std::{collections::HashMap, rc::Rc};

mod constraints;
#[cfg(test)]
mod tests;

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

type SoftConstraintIndex = u8;

#[derive(thiserror::Error, Debug)]
pub enum ScoreManagerError {
    #[error(transparent)]
    ClassCastError(#[from] ObjectCastError),

    #[error(
        "Attempted to cast {0} into a SoftConstraint, but no SoftConstraint exists with said discriminant."
    )]
    SoftConstraintCastError(u8),

    #[error("Attempted to access an out of bounds SoftConstraint, Given i={0} when {1} >= i >= 0")]
    SoftConstraintOutOfBounds(SoftConstraintIndex, SoftConstraintIndex), // given, max

    #[error(
        "Failed to find stored weights of class {0}! This shouldn't ever happen as at the minimum each class gets default weights!"
    )]
    FailedToLocateWeights(String),
}

type ScoreManagerResult<T> = std::result::Result<T, ScoreManagerError>;
type WeightList = Vec<Weight>;

/// An aligned vector to the discriminants of [constraints::SoftConstraints],
struct ConstraintList(HashMap<KnownObject, WeightList>);

impl ConstraintList {
    pub fn new() -> ScoreManagerResult<Self> {
        let mut coll = HashMap::new();

        for class_discrim in 0..OBJECT_COUNT {
            let class: KnownObject = class_discrim.try_into()?;

            let mut weights = Vec::new();

            for weight_type in 0..SOFT_ENUM_VARIANT_COUNT {
                let constraint = transmute_into_constraint(weight_type)?;

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

/// SAFETY:
///     We ensure that idx is within the bounds of [SoftConstraints] since [SOFT_ENUM_VARIANT_COUNT] is a compile-time guarantee
///     To be the amount of variants within [SoftConstraints], we simply check against this guarantee at runtime to ensure
///     safety within the transmutation itself.
///     We have to transmute over just implementing TryFrom<u8> for SoftConstraint since SoftConstriant is defined within a macro.
///     See [src/weighting/mod.rs]
pub fn transmute_into_constraint(idx: SoftConstraintIndex) -> ScoreManagerResult<SoftConstraints> {
    if idx > SOFT_ENUM_VARIANT_COUNT {
        return Err(ScoreManagerError::SoftConstraintCastError(idx));
    }

    let constraint = unsafe { std::mem::transmute::<u8, SoftConstraints>(idx) };
    Ok(constraint)
}

pub struct ScoreManager {
    weighted_constraints: ConstraintList,
    ctx: Rc<Context>,
}

impl ScoreManager {
    pub fn new(ctx: Rc<Context>) -> ScoreManagerResult<Self> {
        Ok(Self {
            weighted_constraints: ConstraintList::new()?,
            ctx,
        })
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
