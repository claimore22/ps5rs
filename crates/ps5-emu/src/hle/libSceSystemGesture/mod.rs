use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    SystemGestureModule,
    "libSceSystemGesture",
    [
        "sceSystemGestureClose",
        "sceSystemGestureCreateTouchRecognizer",
        "sceSystemGestureFinalizePrimitiveTouchRecognizer",
        "sceSystemGestureGetPrimitiveTouchEvents",
        "sceSystemGestureGetTouchEvents",
        "sceSystemGestureInitializePrimitiveTouchRecognizer",
        "sceSystemGestureOpen",
        "sceSystemGestureUpdatePrimitiveTouchRecognizer",
        "sceSystemGestureUpdateTouchRecognizer"
    ]
);
