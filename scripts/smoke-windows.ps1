<#
Smoke test Windows bout-en-bout du vrai exe AutoKey (session interactive requise : SendInput + fenêtres).
Usage : pwsh scripts/smoke-windows.ps1 <chemin de AutoKey.exe>
Sortie : smoke-out/ (captures PNG, journaux). Code retour 1 au premier échec d'étape.
#>
param([Parameter(Mandatory = $true)][string]$Exe)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2

$Exe = (Resolve-Path -LiteralPath $Exe).Path
$OutDir = Join-Path (Get-Location).Path 'smoke-out'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class SmokeWin
{
    [StructLayout(LayoutKind.Sequential)]
    struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Sequential)]
    struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
    [StructLayout(LayoutKind.Explicit)]
    struct InputUnion { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
    [StructLayout(LayoutKind.Sequential)]
    struct INPUT { public uint type; public InputUnion u; }

    delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder sb, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder sb, int n);
    [DllImport("user32.dll")] static extern uint SendInput(uint n, INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] static extern bool BringWindowToTop(IntPtr h);
    [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint a, uint b, bool attach);
    [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();

    public const uint WM_CLOSE = 0x0010;

    // Fenêtres de haut niveau du processus `pid` (cls / title : vide = pas de filtre — PowerShell passe $null en "" ; title = égalité exacte).
    public static IntPtr[] Find(uint pid, string cls, string title, bool visibleOnly)
    {
        List<IntPtr> res = new List<IntPtr>();
        EnumWindows(delegate (IntPtr h, IntPtr l)
        {
            uint p;
            GetWindowThreadProcessId(h, out p);
            if (p != pid) return true;
            if (visibleOnly && !IsWindowVisible(h)) return true;
            if (!string.IsNullOrEmpty(cls))
            {
                StringBuilder c = new StringBuilder(256);
                GetClassName(h, c, 256);
                if (c.ToString() != cls) return true;
            }
            if (!string.IsNullOrEmpty(title))
            {
                StringBuilder t = new StringBuilder(256);
                GetWindowText(h, t, 256);
                if (t.ToString() != title) return true;
            }
            res.Add(h);
            return true;
        }, IntPtr.Zero);
        return res.ToArray();
    }

    // Appui puis relâche de la touche (vk + scancode), ~50 ms entre les deux.
    public static void Tap(ushort vk, ushort scan, int holdMs)
    {
        Send(vk, scan, 0);
        System.Threading.Thread.Sleep(holdMs);
        Send(vk, scan, 2); // KEYEVENTF_KEYUP
    }

    static void Send(ushort vk, ushort scan, uint flags)
    {
        INPUT[] a = new INPUT[1];
        a[0].type = 1; // INPUT_KEYBOARD
        a[0].u.ki.wVk = vk;
        a[0].u.ki.wScan = scan;
        a[0].u.ki.dwFlags = flags;
        uint n = SendInput(1, a, Marshal.SizeOf(typeof(INPUT)));
        if (n != 1) throw new Exception("SendInput a échoué (n=" + n + ", err=" + Marshal.GetLastWin32Error() + ")");
    }

    // Met la fenêtre au premier plan (astuce ALT + AttachThreadInput contre le verrou de premier plan).
    public static bool Foreground(IntPtr h)
    {
        ShowWindow(h, 9); // SW_RESTORE
        keybd_event(0x12, 0x38, 0, UIntPtr.Zero);
        keybd_event(0x12, 0x38, 2, UIntPtr.Zero);
        IntPtr fg = GetForegroundWindow();
        uint dummy;
        uint fgThread = fg == IntPtr.Zero ? 0 : GetWindowThreadProcessId(fg, out dummy);
        uint me = GetCurrentThreadId();
        bool attached = fgThread != 0 && fgThread != me && AttachThreadInput(me, fgThread, true);
        BringWindowToTop(h);
        SetForegroundWindow(h);
        if (attached) AttachThreadInput(me, fgThread, false);
        return GetForegroundWindow() == h;
    }
}
'@

# ------------------------------------------------------------------ utilitaires
$script:results = New-Object System.Collections.ArrayList
$script:failed = $false
$script:app = $null
$script:target = $null
$script:targetHwnd = [IntPtr]::Zero
$script:app2 = $null
$script:finalFail = $true

function Log([string]$m) { Write-Host ("[smoke] " + $m) }

function Shot([string]$name) {
    try {
        $b = [System.Windows.Forms.SystemInformation]::VirtualScreen
        $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($b.Left, $b.Top, 0, 0, $bmp.Size)
        $g.Dispose()
        $bmp.Save((Join-Path $OutDir $name), [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()
        Log "capture $name"
    }
    catch { Log "capture $name impossible : $($_.Exception.Message)" }
}

# Exécute une étape ; la première qui échoue arrête le test (les suivantes sont ignorées, le nettoyage reste fait).
function Step([string]$name, [scriptblock]$body) {
    if ($script:failed) { [void]$script:results.Add([pscustomobject]@{ Step = $name; Result = 'SKIP'; Detail = '' }); return }
    Log "=== $name"
    try {
        & $body
        [void]$script:results.Add([pscustomobject]@{ Step = $name; Result = 'PASS'; Detail = '' })
    }
    catch {
        $script:failed = $true
        Log "ECHEC : $($_.Exception.Message)"
        [void]$script:results.Add([pscustomobject]@{ Step = $name; Result = 'FAIL'; Detail = $_.Exception.Message })
    }
}

function Wait-Until([scriptblock]$cond, [int]$timeoutMs, [int]$pollMs = 100) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $timeoutMs) {
        if (& $cond) { return $true }
        Start-Sleep -Milliseconds $pollMs
    }
    return [bool](& $cond)
}

# Nombre de « e » reçus par la cible (fichier réécrit toutes les 200 ms par la fenêtre WinForms).
function Get-Count {
    $f = Join-Path $script:tmp 'target-out.txt'
    for ($i = 0; $i -lt 10; $i++) {
        try {
            if (-not (Test-Path -LiteralPath $f)) { return 0 }
            $t = [IO.File]::ReadAllText($f)
            return ([regex]::Matches($t, 'e')).Count
        }
        catch { Start-Sleep -Milliseconds 30 }
    }
    throw "Lecture du fichier de la cible impossible"
}

function Test-Focus { return ([SmokeWin]::GetForegroundWindow() -eq $script:targetHwnd) }

# Cible au premier plan, avec une nouvelle tentative ; sinon échec explicite.
function Ensure-Focus {
    if (Test-Focus) { return }
    Log "cible hors premier plan : nouvelle tentative"
    for ($i = 0; $i -lt 3; $i++) {
        [void][SmokeWin]::Foreground($script:targetHwnd)
        Start-Sleep -Milliseconds 300
        if (Test-Focus) { return }
    }
    Shot 'focus-lost.png'
    throw "focus perdu : la fenêtre cible n'est pas au premier plan"
}

function Press-Key([ushort]$vk, [ushort]$scan) { [SmokeWin]::Tap($vk, $scan, 50) }
$F6 = @(0x75, 0x40)
$F8 = @(0x77, 0x42)

function Assert-Alive([string]$what) {
    if ($script:app.HasExited) { throw "$what : AutoKey s'est arrêté (code $($script:app.ExitCode))" }
}

# Après un arrêt (F8 ou bascule F6) : plus aucune touche envoyée.
function Assert-Stopped([string]$what) {
    Start-Sleep -Milliseconds 500
    $n = Get-Count
    Start-Sleep -Milliseconds 1000
    $m = Get-Count
    Log "$what : N=$n puis $m"
    if ($m -gt $n + 1) { throw "$what : des touches arrivent encore après l'arrêt ($n -> $m)" }
}

# ------------------------------------------------------------------ déroulé
$script:tmp = Join-Path ([IO.Path]::GetTempPath()) ("autokey-smoke-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $script:tmp | Out-Null
$cfgDir = Join-Path $script:tmp 'AutoKey'
$cfgFile = Join-Path $cfgDir 'config3.json'

try {
    Step 'a. config minimale' {
        New-Item -ItemType Directory -Force -Path $cfgDir | Out-Null
        # e = 0x45, F6 = 0x75 (bascule spam), F8 = 0x77 (arrêt d'urgence) ; scan 0 = calculé à l'envoi
        $json = @'
{
  "mode": "spam",
  "spam": {
    "key": { "vk": 69, "scan": 0, "ext": false },
    "value": 50.0,
    "unit": "ms",
    "hotkey": [ { "vk": 117, "scan": 0, "ext": false } ],
    "hk_mode": "toggle"
  },
  "stop_hotkey": [ { "vk": 119, "scan": 0, "ext": false } ],
  "auto_check_updates": false,
  "tray": false,
  "start_minimized": false,
  "bind_window": false
}
'@
        [IO.File]::WriteAllText($cfgFile, $json, (New-Object System.Text.UTF8Encoding($false)))
        $null = $json | ConvertFrom-Json
        # APPDATA hérité par le processus lancé (config::config_dir)
        $env:APPDATA = $script:tmp
    }

    Step 'b. démarrage + fenêtre principale' {
        $script:app = Start-Process -FilePath $Exe -PassThru
        $null = $script:app.Handle # garde le handle ouvert (ExitCode fiable sous PowerShell 5)
        $ok = Wait-Until { (-not $script:app.HasExited) -and ([SmokeWin]::Find([uint32]$script:app.Id, $null, 'AutoKey', $true).Count -gt 0) } 15000 250
        if (-not $ok) {
            if ($script:app.HasExited) { throw "AutoKey s'est arrêté au démarrage (code $($script:app.ExitCode))" }
            throw "fenêtre principale « AutoKey » introuvable après 15 s"
        }
        Start-Sleep -Seconds 3
        Assert-Alive 'après 3 s'
        if (Test-Path -LiteralPath (Join-Path $cfgDir 'config3.bad.json')) { throw "config3.bad.json créé : la config de test a été jugée illisible" }
    }

    Step 'c. capture de démarrage' { Shot 'start.png' }

    Step 'd. fenêtre cible (WinForms)' {
        $tscript = @'
param($Out, $Ready)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
$form = New-Object System.Windows.Forms.Form
$form.Text = 'AutoKeySmokeTarget'
$form.StartPosition = 'Manual'
$form.Location = New-Object System.Drawing.Point(40, 40)
$form.Size = New-Object System.Drawing.Size(640, 420)
$tb = New-Object System.Windows.Forms.TextBox
$tb.Multiline = $true
$tb.MaxLength = 1000000
$tb.Dock = 'Fill'
$form.Controls.Add($tb)
$dump = { try { [IO.File]::WriteAllText($Out, $tb.Text) } catch {} }
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 200
$timer.Add_Tick($dump)
$timer.Start()
$form.Add_Shown({
    $form.Activate()
    $tb.Focus() | Out-Null
    [IO.File]::WriteAllText($Ready, [string]$form.Handle.ToInt64())
})
$form.Add_FormClosing($dump)
[void]$form.ShowDialog()
'@
        $tpath = Join-Path $script:tmp 'target.ps1'
        [IO.File]::WriteAllText($tpath, $tscript, (New-Object System.Text.UTF8Encoding($true)))
        $ready = Join-Path $script:tmp 'target-ready.txt'
        $out = Join-Path $script:tmp 'target-out.txt'
        $script:target = Start-Process -FilePath 'powershell.exe' -PassThru -ArgumentList @('-NoProfile', '-STA', '-ExecutionPolicy', 'Bypass', '-File', $tpath, '-Out', $out, '-Ready', $ready)
        if (-not (Wait-Until { Test-Path -LiteralPath $ready } 20000 200)) { throw "fenêtre cible non démarrée après 20 s" }
        Start-Sleep -Milliseconds 300
        $script:targetHwnd = [IntPtr][int64]([IO.File]::ReadAllText($ready).Trim())
        Ensure-Focus
        Log "cible au premier plan (hwnd $($script:targetHwnd))"
    }

    Step 'e. F6 : le spam tape des « e »' {
        Ensure-Focus
        $before = Get-Count
        Press-Key $F6[0] $F6[1]
        Start-Sleep -Seconds 2
        $n = (Get-Count) - $before
        Log "e reçus : $n (attendu ~40)"
        if ($n -lt 10) {
            if (-not (Test-Focus)) {
                # une nouvelle tentative : le spam tourne toujours, on redonne le focus et on recompte
                Log "focus perdu pendant l'envoi : nouvelle tentative"
                Ensure-Focus
                $b2 = Get-Count
                Start-Sleep -Milliseconds 1500
                $n = (Get-Count) - $b2
                if ($n -lt 10) { throw "focus perdu : seulement $n « e » reçus après remise au premier plan" }
            }
            else {
                throw "seulement $n « e » reçus en 2 s (attendu ~40) : raccourci F6 non détecté ou frappes non délivrées"
            }
        }
        Assert-Alive 'pendant le spam'
        Shot 'running.png'
    }

    Step 'f. F8 : arrêt d''urgence' {
        Press-Key $F8[0] $F8[1]
        Assert-Stopped 'F8'
        Assert-Alive 'après F8'
    }

    Step 'g. F6 puis F6 : bascule' {
        Ensure-Focus
        $b = Get-Count
        Press-Key $F6[0] $F6[1]
        Start-Sleep -Seconds 1
        $n = (Get-Count) - $b
        if ($n -lt 5) { throw "redémarrage par F6 : seulement $n « e » en 1 s" }
        Press-Key $F6[0] $F6[1]
        Assert-Stopped 'bascule F6'
    }

    Step 'h. deuxième instance' {
        # Sans icône de zone de notification (tray=false), la 2e instance attend ~8 s puis affiche une
        # MessageBox « déjà en cours » (classe #32770) qui bloque jusqu'à fermeture : fermer la boîte = succès.
        $script:app2 = Start-Process -FilePath $Exe -PassThru
        $null = $script:app2.Handle
        $script:closedBox = $false
        $ok = Wait-Until {
            if ($script:app2.HasExited) { return $true }
            $boxes = [SmokeWin]::Find([uint32]$script:app2.Id, '#32770', $null, $false)
            foreach ($bx in $boxes) {
                Log "MessageBox « déjà en cours » détectée : fermeture"
                Shot 'second-instance-box.png'
                [void][SmokeWin]::PostMessage($bx, [SmokeWin]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
                $script:closedBox = $true
            }
            return $false
        } 20000 250
        if (-not $ok) { throw "la 2e instance ne s'est pas terminée en 20 s" }
        if (-not $script:app2.WaitForExit(5000)) { throw "la 2e instance tourne encore après fermeture de la boîte" }
        Log "2e instance terminée (code $($script:app2.ExitCode), boîte : $($script:closedBox))"
        Assert-Alive '1re instance après la 2e'
    }

    Step 'i. fermeture propre (WM_CLOSE)' {
        $wins = [SmokeWin]::Find([uint32]$script:app.Id, $null, 'AutoKey', $true)
        if ($wins.Count -eq 0) { throw "fenêtre principale introuvable pour WM_CLOSE" }
        [void][SmokeWin]::PostMessage($wins[0], [SmokeWin]::WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
        if (-not $script:app.WaitForExit(10000)) { throw "AutoKey ne s'est pas terminé en 10 s après WM_CLOSE" }
        $code = $script:app.ExitCode
        Log "code de sortie : $code"
        if ($null -ne $code -and $code -ne 0) { throw "code de sortie $code (attendu 0)" }
        if (-not (Test-Path -LiteralPath $cfgFile)) { throw "config3.json a disparu" }
        try { $null = Get-Content -LiteralPath $cfgFile -Raw -Encoding UTF8 | ConvertFrom-Json } catch { throw "config3.json n'est plus un JSON valide : $($_.Exception.Message)" }
        if (Test-Path -LiteralPath (Join-Path $cfgDir 'config3.json.tmp')) { throw "config3.json.tmp orphelin" }
        if (Test-Path -LiteralPath (Join-Path $cfgDir 'config3.bad.json')) { throw "config3.bad.json présent" }
    }
}
finally {
    # j. nettoyage, capture finale, résumé
    Shot 'final.png'
    foreach ($p in @($script:app, $script:app2, $script:target)) {
        if ($null -ne $p) { try { if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue } } catch {} }
    }
    try {
        if (Test-Path -LiteralPath $cfgDir) { Copy-Item -LiteralPath $cfgDir -Destination (Join-Path $OutDir 'AutoKey-config') -Recurse -Force }
        $o = Join-Path $script:tmp 'target-out.txt'
        if (Test-Path -LiteralPath $o) { Copy-Item -LiteralPath $o -Destination (Join-Path $OutDir 'target-out.txt') -Force }
    }
    catch {}
    Write-Host ''
    Write-Host '================ Résumé smoke test ================'
    foreach ($r in $script:results) {
        $line = '{0,-5} {1}' -f $r.Result, $r.Step
        if ($r.Detail) { $line += '  -> ' + $r.Detail }
        Write-Host $line
    }
    $anyFail = ($script:results | Where-Object { $_.Result -eq 'FAIL' } | Measure-Object).Count -gt 0
    $passed = ($script:results | Where-Object { $_.Result -eq 'PASS' } | Measure-Object).Count
    # toutes les étapes a..i doivent avoir réussi (une exception hors étape ne doit pas passer pour un succès)
    $script:finalFail = $anyFail -or ($passed -ne 9)
}

if ($script:finalFail) { Write-Host 'RESULTAT : FAIL'; exit 1 }
Write-Host 'RESULTAT : PASS'
exit 0
