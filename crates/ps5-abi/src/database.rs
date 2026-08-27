use crate::calling_convention::CallingConvention;
use crate::functions::FunctionSignature;
use crate::types::AbiType;

pub fn seed_signatures() -> Vec<FunctionSignature> {
    vec![
        FunctionSignature { name: "sceKernelSleep".to_string(), return_type: AbiType::I32, params: vec![AbiType::U32], variadic: false },
        FunctionSignature { name: "sceKernelUsleep".to_string(), return_type: AbiType::I32, params: vec![AbiType::U32], variadic: false },
        FunctionSignature { name: "sceKernelGetProcessTime".to_string(), return_type: AbiType::U64, params: vec![], variadic: false },
        FunctionSignature { name: "sceKernelGetProcessTimeCounter".to_string(), return_type: AbiType::U64, params: vec![], variadic: false },
        FunctionSignature { name: "scePthreadCreate".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::Ptr, AbiType::Ptr, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "scePthreadJoin".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "scePthreadMutexInit".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "scePthreadMutexLock".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr], variadic: false },
        FunctionSignature { name: "scePthreadMutexUnlock".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr], variadic: false },
        FunctionSignature { name: "scePthreadCondInit".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "scePthreadCondWait".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "scePthreadCondSignal".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr], variadic: false },
        FunctionSignature { name: "malloc".to_string(), return_type: AbiType::Ptr, params: vec![AbiType::U64], variadic: false },
        FunctionSignature { name: "free".to_string(), return_type: AbiType::U32, params: vec![AbiType::Ptr], variadic: false },
        FunctionSignature { name: "memcpy".to_string(), return_type: AbiType::Ptr, params: vec![AbiType::Ptr, AbiType::Ptr, AbiType::U64], variadic: false },
        FunctionSignature { name: "memset".to_string(), return_type: AbiType::Ptr, params: vec![AbiType::Ptr, AbiType::I32, AbiType::U64], variadic: false },
        FunctionSignature { name: "printf".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr], variadic: true },
        FunctionSignature { name: "snprintf".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::U64, AbiType::Ptr], variadic: true },
        FunctionSignature { name: "puts".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr], variadic: false },
        FunctionSignature { name: "sceKernelOpen".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::I32, AbiType::I32], variadic: false },
        FunctionSignature { name: "sceKernelClose".to_string(), return_type: AbiType::I32, params: vec![AbiType::I32], variadic: false },
        FunctionSignature { name: "sceKernelRead".to_string(), return_type: AbiType::I32, params: vec![AbiType::I32, AbiType::Ptr, AbiType::U64], variadic: false },
        FunctionSignature { name: "sceKernelWrite".to_string(), return_type: AbiType::I32, params: vec![AbiType::I32, AbiType::Ptr, AbiType::U64], variadic: false },
        FunctionSignature { name: "sceKernelMmap".to_string(), return_type: AbiType::Ptr, params: vec![AbiType::Ptr, AbiType::U64, AbiType::I32, AbiType::I32, AbiType::I32, AbiType::U64], variadic: false },
        FunctionSignature { name: "sceKernelMunmap".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::U64], variadic: false },
        FunctionSignature { name: "sceVideoOutOpen".to_string(), return_type: AbiType::I32, params: vec![AbiType::I32, AbiType::I32, AbiType::I32, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "scePadCreate".to_string(), return_type: AbiType::I32, params: vec![AbiType::I32], variadic: false },
        FunctionSignature { name: "scePadRead".to_string(), return_type: AbiType::I32, params: vec![AbiType::I32, AbiType::Ptr, AbiType::I32], variadic: false },
        FunctionSignature { name: "sceAudioOutOpen".to_string(), return_type: AbiType::I32, params: vec![AbiType::I32, AbiType::I32, AbiType::I32, AbiType::I32], variadic: false },
        FunctionSignature { name: "sceGnmDrawIndex".to_string(), return_type: AbiType::U32, params: vec![AbiType::Ptr, AbiType::U32, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "sceAgcCreate".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr], variadic: false },
        FunctionSignature { name: "sceSaveDataMount".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "sceHttpCreate".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::I32], variadic: false },
        FunctionSignature { name: "sceNetSend".to_string(), return_type: AbiType::I32, params: vec![AbiType::I32, AbiType::Ptr, AbiType::U64, AbiType::I32], variadic: false },
        FunctionSignature { name: "sceRtcGetCurrentTick".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr], variadic: false },
        FunctionSignature { name: "sceRandomGetRandomNumber".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::U64], variadic: false },
        FunctionSignature { name: "sceDbgLoggingHandler".to_string(), return_type: AbiType::U32, params: vec![AbiType::I32, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "sceDbgSetMinimumLogLevel".to_string(), return_type: AbiType::U32, params: vec![AbiType::I32], variadic: false },
        FunctionSignature { name: "sceKernelAllocateDirectMemory".to_string(), return_type: AbiType::I32, params: vec![AbiType::U64, AbiType::U64, AbiType::U64, AbiType::I32, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "sceKernelMapDirectMemory".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::U64, AbiType::I32, AbiType::I32, AbiType::U64, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "exit".to_string(), return_type: AbiType::U32, params: vec![AbiType::I32], variadic: false },
        FunctionSignature { name: "abort".to_string(), return_type: AbiType::U32, params: vec![], variadic: false },
        FunctionSignature { name: "rand".to_string(), return_type: AbiType::I32, params: vec![], variadic: false },
        FunctionSignature { name: "srand".to_string(), return_type: AbiType::U32, params: vec![AbiType::U32], variadic: false },
        FunctionSignature { name: "qsort".to_string(), return_type: AbiType::U32, params: vec![AbiType::Ptr, AbiType::U64, AbiType::U64, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "strlen".to_string(), return_type: AbiType::U64, params: vec![AbiType::Ptr], variadic: false },
        FunctionSignature { name: "strcmp".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "strcpy".to_string(), return_type: AbiType::Ptr, params: vec![AbiType::Ptr, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "fopen".to_string(), return_type: AbiType::Ptr, params: vec![AbiType::Ptr, AbiType::Ptr], variadic: false },
        FunctionSignature { name: "fclose".to_string(), return_type: AbiType::I32, params: vec![AbiType::Ptr], variadic: false },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_has_50() {
        assert_eq!(seed_signatures().len(), 50);
    }

    #[test]
    fn all_have_names() {
        for sig in seed_signatures() {
            assert!(!sig.name.is_empty());
        }
    }
}
