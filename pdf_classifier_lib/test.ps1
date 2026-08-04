$prefix = "d:/coding/temp_classifier/pdf-classifier-v5/examples/build"

cmake -S . -B ./build -DCMAKE_INSTALL_PREFIX="$prefix" -DCMAKE_PREFIX_PATH="$prefix"

cmake --build ./build --config Release
