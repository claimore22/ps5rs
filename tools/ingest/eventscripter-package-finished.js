/* JDownloader 2 Event Scripter — trigger: "Package finished".
   Calls tools/ingest/ingest-one.ps1 with the finished package folder.
   Serial discipline: JD max simultaneous downloads = 1; the ps1 holds the
   queue by stopping JDownloader on ABORT, and exits 2 to continue on SKIP.
   Paste into Event Scripter, set trigger, adjust PS1 path once. */
(function () {
    if (!package.isFinished()) return;

    var links = package.getDownloadLinks ? package.getDownloadLinks() : [];
    var folder = null;
    if (package.downloadFolder) {
        folder = package.downloadFolder;
    } else if (links.length > 0 && links[0].downloadPath) {
        folder = links[0].downloadPath.substring(0, links[0].downloadPath.lastIndexOf("/"));
    }
    if (!folder) return;

    var ps1 = "C:\\Users\\claimoar\\Documents\\Rust\\ps5rs\\tools\\ingest\\ingest-one.ps1";
    callAsync(function (exitCode, out) {}, "powershell.exe",
        ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", ps1, "-PackageDir", folder]);
})();
