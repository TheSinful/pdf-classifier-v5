#[macro_export]
/// Similar to [debug_assert], just only when the first condition is true.
macro_rules! debug_assert_if {
    ($condition: expr, $assertion: expr, $($arg:tt)+) => {
        #[cfg(debug_assertions)]
        if $condition {
            assert!($assertion, $($arg)*);
        }
    };
}
