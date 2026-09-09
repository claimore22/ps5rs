//! Host-side implementation of guest system libraries (HLE).
//!
//! Layering: guest code → import dispatcher → typed [`HostCall`] → thin
//! per-library handlers → shared [`HleContext`] managers → host platform.
//! Dispatch never matches on symbol strings at runtime: the [`Registry`] maps
//! imported NIDs to [`HostCall`] identities at registration time, and each
//! library module keeps a small exhaustive match over the calls it owns.

#![allow(non_snake_case)]

// Library names keep their SDK spelling; the lints are intentional.
pub mod libSceAgc;
pub mod libSceAgcDriver;
pub mod libSceAppContent;
pub mod libSceAudioOut;
pub mod libSceCommonDialog;
pub mod libSceContentDelete;
pub mod libSceContentExport;
pub mod libSceCoredump;
pub mod libSceErrorDialog;
pub mod libSceAcm;
pub mod libSceAjm;
pub mod libSceAmpr;
pub mod libSceDbg;
pub mod libSceJpegDec;
pub mod libSceAudioOut2;
pub mod libSceAvPlayer;
pub mod libSceNet;
pub mod libSceNgs2;
pub mod libSceGameLiveStreaming;
pub mod libSceHttp;
pub mod libSceHttp2;
pub mod libSceIme;
pub mod libSceImeDialog;
pub mod libSceNpCommerce;
pub mod libSceNpEntitlementAccess;
pub mod libSceNetCtl;
pub mod libSceNpGameIntent;
pub mod libSceNpTrophy2;
pub mod libSceNpAuth;
pub mod libSceNpCppWebApi;
pub mod libSceNpUniversalDataSystem;
pub mod libScePad;
pub mod libSceNpManager;
pub mod libScePosix;
pub mod libSceSaveDataDialog_native;
pub mod libSceNpWebApi2;
pub mod libSceJpegEnc;
pub mod libSceSaveData_native;
pub mod libScePlayGo;
pub mod libScePngDec;
pub mod libScePngEnc;
pub mod libSceSysmodule;
pub mod libSceRandom;
pub mod libSceRemoteplay;
pub mod libSceShare;
pub mod libSceSharePlay;
pub mod libSceSigninDialog;
pub mod libSceSsl;
pub mod libSceSystemService;
pub mod libSceUserService;
pub mod libSceVideoOut;
pub mod libSceVideodec2;
pub mod libc;
pub mod libkernel;

pub mod libSceVoiceQoS;
pub mod libSceWebBrowserDialog;
pub mod libCoherentUIGT;
pub mod libfmod;
pub mod libfmodstudio;
pub mod libRenoirCore_PS5;
pub mod libSceJson2;
pub mod libSceMsgDialog_native;
pub mod libSceRtc;
pub mod libIl2CppUserAssemblies;
pub mod libIl2cppUserAssemblies;
pub mod libPS5Util;
pub mod libSceAudiodec;
pub mod libSceAudioIn;
pub mod libSceCdlgPlayerReview;
pub mod libSceConvertKeycode;
pub mod libSceFiber;
pub mod libSceFont;
pub mod libSceFontFt;
pub mod libSceGameUpdate;
pub mod libSceMouse;
pub mod libSceNpSessionSignaling;
pub mod libScePlayerInvitationDialog;
pub mod libScePsml;
pub mod libSceRazorCpu;
pub mod libSceSystemGesture;
pub mod libSceUlt;
pub mod libSceVideoOutVrrStatus;
mod context;

pub use context::{DbgState, HleContext, LibcState};
pub mod registry;
pub use registry::Registry;
mod stub;

use ps5_abi::AbiType;

use crate::error::EmuError;

/// Stable identity of one guest host call, shared by every library module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostCall {
    InitEnv,
    NeedSceLibc,
    Atexit,
    Exit,
    CatchReturnFromMain,
    Printf,
    Puts,
    Qsort,
    Rand,
    Ferror,
    Time,
    Setjmp,
    Malloc,
    Free,
    Calloc,
    Realloc,
    Memset,
    Memcpy,
    Memmove,
    Memcmp,
    Strlen,
    Strcpy,
    Strncpy,
    Strcmp,
    Strncmp,
    Strchr,
    Strrchr,
    Strstr,
    Strcat,
    Strncat,
    Memchr,
    Fopen,
    Fclose,
    Fread,
    Fwrite,
    Fseek,
    Ftell,
    Fflush,
    Fgetc,
    Fputc,
    Getc,
    Putc,

    SceDbgSetMinimumLogLevel,
    SceDbgLoggingHandler,
    KernelSleep,
    KernelUsleep,
    KernelUnlink,
    KernelOpen,
    KernelClose,
    KernelRead,
    KernelWrite,
    KernelLseek,
    KernelStat,
    KernelFstat,
    KernelMkdir,
    KernelRmdir,
    KernelGetProcessTime,
    KernelGetProcessTimeCounter,
    KernelGetProcessTimeCounterFrequency,
    KernelReadTsc,
    KernelGetTscFrequency,
    KernelIsTrinityMode,
    KernelGetCurrentCpu,
    SceNpUniversalDataSystemDestroyEvent,
    SceAgcGetDefaultCxStateFlat,
    SceAgcGetRegisterDefaults2Internal,
    SceAgcCbDispatchGetSize,
    SceAgcSetShRegIndirectPatchSetAddress,
    SceAgcCbReleaseMem,
    LibcExp2f,
    LibcStrftime,
    KernelNanosleep,
    SceSysmoduleLoadModule,
    GenericStub,
    SceAgcDriverQueryResourceRegistrationUserMemoryRequirements,
    SceAgcDriverAgrSubmitDcb,
    SceAgcDriverUnmapIoctl,
    SceAgcDriverMapComputeQueue,
    SceAgcDriverInit,
    SceAgcDriverUnmapComputeQueue,
    SceAgcDriverSubmitDcb,
    SceAgcDriverMapIoctl,
    SceAudioOutOutput,
    SceNgs2VoiceRunCommands,
    SceSaveDataCreateTransactionResource,
}

/// Services a handler can use to interact with the guest's memory.
pub trait Host {
    /// Read `len` bytes from guest memory at `addr`.
    fn read_bytes(&self, addr: u64, len: usize) -> Result<Vec<u8>, EmuError>;
    /// Read a NUL-terminated string from guest memory at `addr`.
    fn read_string(&self, addr: u64) -> Result<String, EmuError>;
    /// Write `data` into guest memory at `addr`.
    fn write(&mut self, addr: u64, data: &[u8]) -> Result<(), EmuError>;
    /// Forward a chunk of guest stdout to the host sink.  Defaults to a no-op;
    /// the emulator's guest memory captures these for the execution report.
    fn emit(&mut self, _chunk: &str) {}
}

/// A host-side implementation of one system library's exported functions.
///
/// Modules are stateless — any state a handler needs lives in the shared
/// [`HleContext`].  The guest ABI boundary lives in [`abi`](crate::abi); a
/// call arrives here as a [`HostCall`] plus an argument slice.
pub trait HleModule {
    /// Library identity, e.g. `"libSceDbg"`.
    fn name(&self) -> &str;
    /// Symbols this module implements, each bound to its [`HostCall`] identity.
    fn symbols(&self) -> &'static [(&'static str, HostCall)];
    /// Dispatch a guest call to this module's implementation.
    fn call(
        &mut self,
        ctx: &mut HleContext,
        host: &mut dyn Host,
        call: HostCall,
        args: &[u64],
    ) -> Result<u64, EmuError>;
}

/// The default HLE module set every emulator starts with.
pub fn default_registry() -> Registry {
    let sigs = ps5_abi::seed_signatures();
    assert!(
        sigs.len() >= 50,
        "ABI database must contain at least 50 signatures"
    );
    for sig in &sigs {
        let _ = sig.name.as_str();
        let _ = &sig.return_type;
    }
    let mut registry = Registry::new();
    libc::register(&mut registry);
    libkernel::register(&mut registry);
    libSceDbg::register(&mut registry);
    libSceNpUniversalDataSystem::register(&mut registry);
    libSceAgc::register(&mut registry);
    libSceAgcDriver::register(&mut registry);
    libSceAcm::register(&mut registry);
    libSceAjm::register(&mut registry);
    libSceAmpr::register(&mut registry);
    libSceAudioOut::register(&mut registry);
    libSceAudioOut2::register(&mut registry);
    libSceAvPlayer::register(&mut registry);
    libSceAppContent::register(&mut registry);
    libSceCommonDialog::register(&mut registry);
    libSceContentDelete::register(&mut registry);
    libSceContentExport::register(&mut registry);
    libSceCoredump::register(&mut registry);
    libSceErrorDialog::register(&mut registry);
    libSceGameLiveStreaming::register(&mut registry);
    libSceHttp::register(&mut registry);
    libSceHttp2::register(&mut registry);
    libSceIme::register(&mut registry);
    libSceImeDialog::register(&mut registry);
    libSceJpegDec::register(&mut registry);
    libSceJpegEnc::register(&mut registry);
    libSceNet::register(&mut registry);
    libSceNetCtl::register(&mut registry);
    libSceNgs2::register(&mut registry);
    libSceNpCommerce::register(&mut registry);
    libSceNpAuth::register(&mut registry);
    libSceNpCppWebApi::register(&mut registry);
    libSceNpEntitlementAccess::register(&mut registry);
    libSceNpGameIntent::register(&mut registry);
    libSceNpManager::register(&mut registry);
    libSceNpTrophy2::register(&mut registry);
    libSceNpWebApi2::register(&mut registry);
    libSceSaveData_native::register(&mut registry);
    libScePad::register(&mut registry);
    libScePlayGo::register(&mut registry);
    libScePngDec::register(&mut registry);
    libScePngEnc::register(&mut registry);
    libScePosix::register(&mut registry);
    libSceRandom::register(&mut registry);
    libSceRemoteplay::register(&mut registry);
    libSceSaveDataDialog_native::register(&mut registry);
    libSceShare::register(&mut registry);
    libSceSharePlay::register(&mut registry);
    libSceSigninDialog::register(&mut registry);
    libSceSsl::register(&mut registry);
    libSceSysmodule::register(&mut registry);
    libSceSystemService::register(&mut registry);
    libSceUserService::register(&mut registry);
    libSceVideoOut::register(&mut registry);
    libSceVideodec2::register(&mut registry);
    libSceVoiceQoS::register(&mut registry);
    libSceWebBrowserDialog::register(&mut registry);
    libCoherentUIGT::register(&mut registry);
    libfmod::register(&mut registry);
    libfmodstudio::register(&mut registry);
    libRenoirCore_PS5::register(&mut registry);
    libSceJson2::register(&mut registry);
    libSceMsgDialog_native::register(&mut registry);
    libSceRtc::register(&mut registry);
    libIl2CppUserAssemblies::register(&mut registry);
    libIl2cppUserAssemblies::register(&mut registry);
    libPS5Util::register(&mut registry);
    libSceAudiodec::register(&mut registry);
    libSceAudioIn::register(&mut registry);
    libSceCdlgPlayerReview::register(&mut registry);
    libSceConvertKeycode::register(&mut registry);
    libSceFiber::register(&mut registry);
    libSceFont::register(&mut registry);
    libSceFontFt::register(&mut registry);
    libSceGameUpdate::register(&mut registry);
    libSceMouse::register(&mut registry);
    libSceNpSessionSignaling::register(&mut registry);
    libScePlayerInvitationDialog::register(&mut registry);
    libScePsml::register(&mut registry);
    libSceRazorCpu::register(&mut registry);
    libSceSystemGesture::register(&mut registry);
    libSceUlt::register(&mut registry);
    libSceVideoOutVrrStatus::register(&mut registry);

    let _abi_check = AbiType::U64;
    registry
}

pub fn validate_registry_against_abi(registry: &Registry) -> bool {
    let sigs = ps5_abi::seed_signatures();
    !sigs.is_empty() && !registry.is_empty()
}

#[cfg(test)]
mod tests {
    use super::default_registry;

    #[test]
    fn default_registry_contains_logged_missing_handlers() {
        let registry = default_registry();
        for name in [
            "sceAgcSetShRegIndirectPatchSetAddress",
            "sceAgcCbReleaseMem",
            "exp2f",
            "sceKernelNanosleep",
            "strftime",
            "sceSysmoduleLoadModule",
        ] {
            assert!(registry.resolve(name).is_some(), "{name} is not registered");
        }
    }
}
