pub mod illegal_second_in_pair;

use super::impl_constraint_enum;
use crate::constraints::Constraint;
use crate::context::Context;
use crate::generated::generated_object_types::KnownObject;
use crate::page::Page;
use illegal_second_in_pair::IllegalSecondInPair;

/// HardConstraints are a type of Constraint that forces a class, 'x' out of inference.
///
/// By returning false, 'x' is taken out of the race on page 'n' guaranteed.
trait HardConstraint: Constraint {
    /// class, 'x' cannot be page, 'n' guaranteed.
    #[allow(non_snake_case)]
    fn FAIL() -> bool {
        false
    }

    /// class, 'x' may still be applicable for page, 'n'.
    #[allow(non_snake_case)]
    fn PASS() -> bool {
        true
    }

    fn eval(ctx: &Context, class: KnownObject, page: Page) -> bool;
}

impl_constraint_enum!(HardConstraints, bool, IllegalSecondInPair = IllegalSecondInPair);
