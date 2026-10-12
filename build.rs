fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-env-changed=AUTOKEY_BUILD");
    println!("cargo:rerun-if-env-changed=AUTOKEY_REPO");
    #[cfg(windows)]
    {
        // Manifeste : droits utilisateur normaux (jamais d'élévation admin), DPI par écran, Windows 10/11
        const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="AutoKey" version="1.0.0.0" processorArchitecture="*"/>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security><requestedPrivileges>
      <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
    </requestedPrivileges></security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application><supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/></application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
  <dependency><dependentAssembly>
    <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
      processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
  </dependentAssembly></dependency>
</assembly>"#;
        let build = std::env::var("AUTOKEY_BUILD").unwrap_or_else(|_| "0".into());
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.set_manifest(MANIFEST);
        res.set("FileDescription", "AutoKey — auto-presser clavier");
        res.set("ProductName", "AutoKey");
        res.set("OriginalFilename", "AutoKey.exe");
        res.set("InternalName", "AutoKey");
        res.set("Comments", &format!("build {build}"));
        res.set("CompanyName", "sneakyquaza");
        res.set("LegalCopyright", "© 2026 sneakyquaza — licence MIT");
        // numéros de version cohérents (3.1.0.<build>) : un exe sans métadonnées paraît plus suspect
        let ver = env!("CARGO_PKG_VERSION");
        let mut parts = ver.split('.').map(|x| x.parse::<u64>().unwrap_or(0));
        let (ma, mi, pa) = (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0));
        let b: u64 = build.parse().unwrap_or(0).min(65535);
        let packed = (ma << 48) | (mi << 32) | (pa << 16) | b;
        res.set_version_info(winresource::VersionInfo::FILEVERSION, packed);
        res.set_version_info(winresource::VersionInfo::PRODUCTVERSION, packed);
        res.set("FileVersion", &format!("{ver}.{b}"));
        res.set("ProductVersion", &format!("{ver}.{b}"));
        if let Err(e) = res.compile() {
            println!("cargo:warning=ressources Windows non intégrées : {e}");
        }
    }
}
