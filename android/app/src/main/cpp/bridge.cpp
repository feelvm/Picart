// JNI glue: include/com_picsartclone_engine_EditorEngine.h equivalent.
// Implemented in android/app/src/main/cpp/bridge.cpp:
//
// #include <jni.h>
// // each method below forwards to ec_* in libeditor_core.so and returns jstring
// // e.g. Java_com_picsartclone_engine_EditorEngine_renderPreview {
// //   char* p = ec_render_preview(w, h, interacting);
// //   jstring s = env->NewStringUTF(p); ec_free_string(p); return s; }
//
// Build with cargo-ndk + CMake; decode via AImageDecoder / HardwareBuffer,
// encode via MediaCodec; NNAPI/ONNX bg-removal posts to the AI thread.
