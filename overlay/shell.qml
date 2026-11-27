//@ pragma UseQApplication
//@ pragma Env QS_NO_RELOAD_POPUP=1

import Quickshell
import Quickshell.Io

// Ghosthand's overlay. Run it with `qs -c ghosthand` (or `ghosthand overlay`) and leave it running.
ShellRoot {
    Overlay {}

    // The client side: `qs -c ghosthand ipc call ghosthand <function>`.
    IpcHandler {
        target: "ghosthand"
        // "allowed" (a standing grant, or the gate is off) or "asking": then poll status.
        function ask(who: string, reason: string, seconds: int, scope: string, detail: string): string { return Gate.ask(who, reason, seconds, scope, detail); }
        // idle | asking | active | allowed | denied | timeout | revoked
        function status(): string { return Gate.status(); }
        function begin(who: string): void { Gate.begin(who); }
        function end(): void { Gate.end(); }
        function allow(minutes: int): void { Gate.allow(minutes); }
        function deny(): void { Gate.deny(); }
        function revoke(): void { Gate.revoke(); }
    }
}
