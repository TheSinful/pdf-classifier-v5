use paste::paste;
use std::path::PathBuf;

#[ctor::ctor]
fn init_tests() {
    init_logger();
    set_SMALL_TEST_DOC_test_path();
    set_LARGE_TEST_DOC_test_path();
    set_TEST_OUTPUT_DIR_test_path();
}

macro_rules! get_root_path {
    () => {
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
    };
}

macro_rules! def_test_path {
    ($name: ident, $path: expr) => {
        paste! {
            const $name: &'static str = $path;

            #[allow(non_snake_case)]
            fn [<set_ $name _test_path>]() -> () {
                unsafe {
                    std::env::set_var(
                        stringify!($name),
                        get_root_path!().join(stringify!($path))
                    )
                }
            }

            #[allow(non_snake_case)]
            pub fn [<get_ $name _test_path>]() -> PathBuf {
                get_root_path!().join($path)
            }
        }
    };

    ($name: ident, $path: expr, $dir: expr) => {
        paste! {
            const $name: &'static str = $path;

            #[allow(non_snake_case)]
            fn [<set_ $name _test_path>]() -> () {
                let path = get_root_path!().join($path);

                if !path.exists() {
                    std::fs::create_dir(path.clone()).expect(&format!("failed to create test directory {} at path {}", stringify!($name), path.clone().to_string_lossy().to_string()));
                }

                unsafe {
                    std::env::set_var(
                        stringify!($name),
                        path
                    )
                }
            }

            #[allow(non_snake_case)]
            pub fn [<get_ $name _test_path>]() -> PathBuf {
                get_root_path!().join($path)
            }
        }
    };
}

def_test_path!(SMALL_TEST_DOC, "../data/small_test_doc.pdf");
def_test_path!(LARGE_TEST_DOC, "../data/large_test_doc.pdf");
def_test_path!(TEST_OUTPUT_DIR, "target/classifier_tests", true);


fn init_logger() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .is_test(true)
        .init();
}
