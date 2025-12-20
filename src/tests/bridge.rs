use crate::initializer::*;

#[test]
fn create_ctx() {
    let ctx = unsafe { Context::new(STANDARD_MEM_LIMIT) };
    assert!(!ctx.0.is_null())
}
