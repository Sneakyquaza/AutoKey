//! Mise à jour automatique depuis les Releases GitHub du dépôt qui a compilé l'exe.
//! Sécurité : HTTPS uniquement (rustls), seules les URL du dépôt sont acceptées,
//! taille limitée, empreinte SHA-256 vérifiée, signature minisign vérifiée (clé publique
//! des releases intégrée), en-tête « MZ » vérifié,
//! remplacement atomique avec retour arrière en cas d'échec.
use crate::control::Shared;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Renseignés par la CI GitHub (owner/repo et numéro de build). Absents en build local → pas de mise à jour.
pub const REPO: Option<&str> = option_env!("AUTOKEY_REPO");
pub const BUILD: Option<&str> = option_env!("AUTOKEY_BUILD");

const EXE_ASSET: &str = "AutoKey.exe";
const SHA_ASSET: &str = "AutoKey.exe.sha256";
const SIG_ASSET: &str = "AutoKey.exe.minisig";
/// Clé publique minisign des releases (la clé privée est dans le secret GitHub MINISIGN_KEY).
const PUBKEY: &str = "RWRpMB1oOVO8oAhJTqnFniOXTkaykl7/VHz7aPE2WQodN+jhG6DtcX6V";
const MAX_EXE: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum UpState {
    Idle,
    Available { build: u64, version: Option<String>, notes: Vec<(String, String)>, exe_url: String, sha_url: String, sig_url: String },
    Downloading(u8),
    Failed(&'static str),
    Restarting,
}

impl From<Release> for UpState {
    fn from(r: Release) -> Self {
        UpState::Available { build: r.build, version: r.version, notes: r.notes, exe_url: r.exe_url, sha_url: r.sha_url, sig_url: r.sig_url }
    }
}

/// Résultat de la vérification manuelle (panneau Paramètres)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Check {
    Idle,
    Busy,
    UpToDate,
    Failed(&'static str),
}

pub struct Updater {
    pub state: Mutex<UpState>,
    pub check: Mutex<Check>,
    pub quit: AtomicBool,
    sh: Arc<Shared>,
}

pub fn current_build() -> u64 {
    BUILD.and_then(|b| b.trim().parse().ok()).unwrap_or(0)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .https_only(true)
        .try_proxy_from_env(true) // respecte le proxy système (HTTPS_PROXY) s'il y en a un
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .redirects(5)
        .user_agent(concat!("AutoKey-updater/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// Release plus récente trouvée sur GitHub
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub build: u64,
    /// Version affichable (« 3.4.1 »), lue dans le titre de la release ; None pour les anciennes
    pub version: Option<String>,
    /// Notes (code de langue « fr »/« en »/« pt », texte), max 6 par langue
    pub notes: Vec<(String, String)>,
    pub exe_url: String,
    pub sha_url: String,
    pub sig_url: String,
}

/// « AutoKey 3.4.1 (build 58) » → « 3.4.1 » (chiffres et points, 16 caractères max)
fn parse_version(name: &str) -> Option<String> {
    let w = name.strip_prefix("AutoKey ")?.split_whitespace().next()?;
    let ok = !w.is_empty() && w.len() <= 16 && w.chars().all(|c| c.is_ascii_digit() || c == '.') && w.chars().any(|c| c.is_ascii_digit());
    ok.then(|| w.to_string())
}

/// Lignes « - [fr] texte » du corps de la release (texte nettoyé, 140 caractères, 6 lignes par langue)
fn parse_notes(body: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in body.lines() {
        let Some(rest) = line.trim().strip_prefix("- [") else { continue };
        let Some((code, txt)) = rest.split_once(']') else { continue };
        if !matches!(code, "fr" | "en" | "pt") {
            continue;
        }
        let clean: String = txt.chars().filter(|c| !c.is_control()).collect();
        let clean: String = clean.trim().chars().take(140).collect();
        if clean.is_empty() || out.iter().filter(|(c, _)| c == code).count() >= 6 {
            continue;
        }
        out.push((code.to_string(), clean));
    }
    out
}

/// Analyse la réponse JSON de /releases/latest. Renvoie la release si elle est plus récente.
pub fn parse_release(json: &str, repo: &str, cur: u64) -> Option<Release> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    if v.get("draft").and_then(|d| d.as_bool()) == Some(true) || v.get("prerelease").and_then(|d| d.as_bool()) == Some(true) {
        return None;
    }
    let tag = v.get("tag_name")?.as_str()?;
    let build: u64 = tag.strip_prefix("build-")?.parse().ok()?;
    if build <= cur {
        return None;
    }
    let prefix = format!("https://github.com/{repo}/releases/download/");
    let mut exe = None;
    let mut sha = None;
    let mut sig = None;
    for a in v.get("assets")?.as_array()? {
        let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let url = a.get("browser_download_url").and_then(|n| n.as_str()).unwrap_or("");
        if !url.starts_with(&prefix) {
            continue; // on n'accepte que les fichiers de CE dépôt
        }
        match name {
            EXE_ASSET => exe = Some(url.to_string()),
            SHA_ASSET => sha = Some(url.to_string()),
            SIG_ASSET => sig = Some(url.to_string()),
            _ => {}
        }
    }
    let version = v.get("name").and_then(|n| n.as_str()).and_then(parse_version);
    let notes = v.get("body").and_then(|b| b.as_str()).map(parse_notes).unwrap_or_default();
    Some(Release { build, version, notes, exe_url: exe?, sha_url: sha?, sig_url: sig? })
}

pub fn parse_sha(txt: &str) -> Option<[u8; 32]> {
    // premier mot de 64 caractères hexadécimaux (formats « hash  fichier » ou avec en-tête)
    let hex = txt.split(|c: char| !c.is_ascii_hexdigit()).find(|w| w.len() == 64)?;
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

fn old_path(exe: &Path) -> PathBuf {
    exe.with_extension("old")
}

/// Le fichier téléchargé doit avoir exactement l'empreinte publiée et être un exécutable Windows
pub fn verify(data: &[u8], got: &[u8; 32], expected: &[u8; 32]) -> Result<(), &'static str> {
    if got != expected || data.len() < 1024 || &data[..2] != b"MZ" {
        return Err("up_bad");
    }
    Ok(())
}

/// La signature minisign (fichier .minisig) doit correspondre aux données et à la clé publique
pub fn verify_sig(data: &[u8], sig_txt: &str, pubkey_b64: &str) -> Result<(), &'static str> {
    let pk = minisign_verify::PublicKey::from_base64(pubkey_b64).map_err(|_| "up_bad")?;
    let sig = minisign_verify::Signature::decode(sig_txt).map_err(|_| "up_bad")?;
    pk.verify(data, &sig, false).map_err(|_| "up_bad")
}

/// Remplace l'exe : actuel → .old, nouveau → nom d'origine. Retour arrière si une étape échoue.
pub fn swap_exe(exe: &Path, data: &[u8]) -> Result<(), &'static str> {
    let tmp = exe.with_extension("update.tmp");
    std::fs::write(&tmp, data).map_err(|_| "up_io")?;
    let old = old_path(exe);
    let _ = std::fs::remove_file(&old);
    if std::fs::rename(exe, &old).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return Err("up_io");
    }
    if std::fs::rename(&tmp, exe).is_err() {
        let _ = std::fs::rename(&old, exe);
        let _ = std::fs::remove_file(&tmp);
        return Err("up_io");
    }
    Ok(())
}

impl Updater {
    pub fn new(sh: Arc<Shared>) -> Arc<Self> {
        Arc::new(Updater { state: Mutex::new(UpState::Idle), check: Mutex::new(Check::Idle), quit: AtomicBool::new(false), sh })
    }

    pub fn state(&self) -> UpState {
        self.state.lock().unwrap().clone()
    }

    fn set(&self, s: UpState) {
        *self.state.lock().unwrap() = s;
        self.sh.repaint();
    }

    /// Interroge /releases/latest : `Some(..)` si une version plus récente existe
    fn fetch_latest(repo: &str) -> Result<Option<Release>, &'static str> {
        let url = format!("https://api.github.com/repos/{repo}/releases/latest");
        let resp = agent().get(&url).set("Accept", "application/vnd.github+json").call().map_err(|_| "up_net")?;
        let body = resp.into_string().map_err(|_| "up_net")?;
        Ok(parse_release(&body, repo, current_build()))
    }

    /// Nettoie l'ancien exe après une mise à jour ; si `auto`, vérifie au démarrage puis toutes les 6 h.
    pub fn spawn_checker(self: &Arc<Self>, auto: bool) {
        let me = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(4));
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::fs::remove_file(old_path(&exe));
            }
            let (Some(repo), true, true) = (REPO, current_build() > 0, auto) else { return };
            loop {
                if matches!(me.state(), UpState::Idle | UpState::Failed(_)) {
                    if let Ok(Some(r)) = Self::fetch_latest(repo) {
                        me.set(r.into());
                    }
                }
                std::thread::sleep(Duration::from_secs(6 * 3600));
            }
        });
    }

    /// Vérification à la demande (bouton « Vérifier maintenant »), résultat dans `check`
    pub fn check_now(self: &Arc<Self>) {
        {
            let mut c = self.check.lock().unwrap();
            if *c == Check::Busy || !matches!(self.state(), UpState::Idle | UpState::Failed(_)) {
                return;
            }
            *c = Check::Busy;
        }
        let me = self.clone();
        std::thread::spawn(move || {
            let res = match REPO {
                Some(repo) if current_build() > 0 => Self::fetch_latest(repo),
                _ => Err("up_na"),
            };
            let c = match res {
                Ok(Some(r)) => {
                    me.set(r.into());
                    Check::Idle
                }
                Ok(None) => Check::UpToDate,
                Err(e) => Check::Failed(e),
            };
            *me.check.lock().unwrap() = c;
            me.sh.repaint();
        });
    }

    pub fn install(self: &Arc<Self>) {
        let UpState::Available { exe_url, sha_url, sig_url, .. } = self.state() else { return };
        self.set(UpState::Downloading(0));
        let me = self.clone();
        std::thread::spawn(move || {
            if let Err(e) = me.do_install(&exe_url, &sha_url, &sig_url) {
                me.set(UpState::Failed(e));
            }
        });
    }

    fn do_install(&self, exe_url: &str, sha_url: &str, sig_url: &str) -> Result<(), &'static str> {
        let ag = agent();
        // 1) empreinte attendue
        let sha_txt = ag.get(sha_url).call().map_err(|_| "up_net")?.into_string().map_err(|_| "up_net")?;
        let expected = parse_sha(&sha_txt).ok_or("up_bad")?;
        // signature minisign (petit fichier texte, taille bornée)
        let mut sig_txt = String::new();
        ag.get(sig_url).call().map_err(|_| "up_net")?.into_reader().take(4096).read_to_string(&mut sig_txt).map_err(|_| "up_net")?;
        // 2) téléchargement (taille limitée)
        let resp = ag.get(exe_url).call().map_err(|_| "up_net")?;
        if !resp.get_url().starts_with("https://") {
            return Err("up_bad");
        }
        let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
        if total > MAX_EXE {
            return Err("up_bad");
        }
        let mut reader = resp.into_reader().take(MAX_EXE + 1);
        let mut data = Vec::with_capacity(total as usize);
        let mut buf = [0u8; 64 * 1024];
        let mut hasher = Sha256::new();
        loop {
            let n = reader.read(&mut buf).map_err(|_| "up_net")?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            data.extend_from_slice(&buf[..n]);
            if data.len() as u64 > MAX_EXE {
                return Err("up_bad");
            }
            if let Some(pct) = (data.len() as u64 * 100).checked_div(total) {
                self.set(UpState::Downloading(pct.min(99) as u8));
            }
        }
        // 3) vérifications
        let got: [u8; 32] = hasher.finalize().into();
        verify(&data, &got, &expected)?;
        verify_sig(&data, &sig_txt, PUBKEY)?;
        // 4) remplacement : exe actuel → .old, nouveau → nom d'origine (retour arrière si échec)
        let exe = std::env::current_exe().map_err(|_| "up_io")?;
        swap_exe(&exe, &data)?;
        let old = old_path(&exe);
        let tmp = exe.with_extension("update.tmp");
        // 5) on sauvegarde, on relance la nouvelle version et on quitte
        self.sh.runner.stop();
        self.sh.save_now();
        if std::process::Command::new(&exe).spawn().is_err() {
            let _ = std::fs::rename(&exe, &tmp);
            let _ = std::fs::rename(&old, &exe);
            return Err("up_io");
        }
        self.set(UpState::Restarting);
        self.quit.store(true, SeqCst);
        self.sh.repaint();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REL: &str = r#"{"tag_name":"build-57","draft":false,"prerelease":false,"assets":[
        {"name":"AutoKey.exe","browser_download_url":"https://github.com/me/AutoKey/releases/download/build-57/AutoKey.exe"},
        {"name":"AutoKey.exe.sha256","browser_download_url":"https://github.com/me/AutoKey/releases/download/build-57/AutoKey.exe.sha256"},
        {"name":"AutoKey.exe.minisig","browser_download_url":"https://github.com/me/AutoKey/releases/download/build-57/AutoKey.exe.minisig"}]}"#;

    #[test]
    fn newer_release_detected() {
        let r = parse_release(REL, "me/AutoKey", 42).unwrap();
        assert_eq!(r.build, 57);
        assert!(r.exe_url.ends_with("/AutoKey.exe"));
        assert!(r.sig_url.ends_with("/AutoKey.exe.minisig"));
        assert_eq!(r.version, None);
        assert!(r.notes.is_empty(), "body absent : pas de notes");
        assert!(parse_release(REL, "me/AutoKey", 57).is_none(), "même build : rien");
    }

    #[test]
    fn draft_prerelease_and_malformed_ignored() {
        let draft = REL.replace("\"draft\":false", "\"draft\":true");
        assert!(parse_release(&draft, "me/AutoKey", 1).is_none());
        let pre = REL.replace("\"prerelease\":false", "\"prerelease\":true");
        assert!(parse_release(&pre, "me/AutoKey", 1).is_none());
        // tag qui n'est pas « build-N » (ex. tag de version) : ignoré
        assert!(parse_release(&REL.replace("build-57\",", "v3.3.1\","), "me/AutoKey", 1).is_none());
        assert!(parse_release(&REL.replace("build-57\",", "build-x\","), "me/AutoKey", 1).is_none());
        for bad in ["", "{", "[]", "null", r#"{"tag_name":"build-9"}"#, r#"{"tag_name":"build-9","assets":[]}"#] {
            assert!(parse_release(bad, "me/AutoKey", 1).is_none(), "{bad}");
        }
    }

    #[test]
    fn foreign_urls_rejected() {
        assert!(parse_release(REL, "pirate/AutoKey", 1).is_none());
        let evil = REL.replace("https://github.com/me", "https://evil.example/me");
        assert!(parse_release(&evil, "me/AutoKey", 1).is_none());
    }

    #[test]
    fn release_without_minisig_rejected() {
        let no_sig = REL.replace("AutoKey.exe.minisig\"", "autre.txt\"");
        assert!(parse_release(&no_sig, "me/AutoKey", 1).is_none());
    }

    const FX_DATA: &[u8] = include_bytes!("../tests/fixtures/update/test.exe.bin");
    const FX_SIG: &str = include_str!("../tests/fixtures/update/test.exe.bin.minisig");
    const FX_PUB: &str = include_str!("../tests/fixtures/update/test.pub");

    fn fx_key() -> &'static str {
        FX_PUB.lines().nth(1).unwrap().trim()
    }

    #[test]
    fn signature_valid() {
        assert_eq!(&FX_DATA[..2], b"MZ");
        assert!(verify_sig(FX_DATA, FX_SIG, fx_key()).is_ok());
    }

    #[test]
    fn signature_rejects_tampering() {
        let mut bad = FX_DATA.to_vec();
        bad[100] ^= 1;
        assert_eq!(verify_sig(&bad, FX_SIG, fx_key()), Err("up_bad"), "donnée modifiée");
        assert_eq!(verify_sig(FX_DATA, FX_SIG, PUBKEY), Err("up_bad"), "mauvaise clé");
        assert_eq!(verify_sig(FX_DATA, "n'importe quoi", fx_key()), Err("up_bad"), "signature illisible");
    }

    #[test]
    fn verify_and_swap() {
        let mut data = vec![0u8; 4096];
        data[0] = b'M';
        data[1] = b'Z';
        let good: [u8; 32] = Sha256::digest(&data).into();
        assert!(verify(&data, &good, &good).is_ok());
        assert_eq!(verify(&data, &good, &[0; 32]), Err("up_bad"), "empreinte différente refusée");
        let mut not_exe = data.clone();
        not_exe[0] = b'X';
        let h: [u8; 32] = Sha256::digest(&not_exe).into();
        assert_eq!(verify(&not_exe, &h, &h), Err("up_bad"), "pas un .exe refusé");

        let dir = std::env::temp_dir().join(format!("autokey-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let exe = dir.join("AutoKey.exe");
        std::fs::write(&exe, b"ancienne version").unwrap();
        swap_exe(&exe, &data).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), data);
        assert_eq!(std::fs::read(old_path(&exe)).unwrap(), b"ancienne version");
        assert!(!exe.with_extension("update.tmp").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sha_parsing() {
        let h = "a".repeat(64);
        assert_eq!(parse_sha(&format!("{h}  AutoKey.exe\n")).unwrap(), [0xaa; 32]);
        assert!(parse_sha("zz").is_none());
        let other = format!("SHA256 hash of x.zip:\r\n{}", "b".repeat(64));
        assert_eq!(parse_sha(&other).unwrap(), [0xbb; 32]);
    }

    #[test]
    fn version_from_name() {
        let with = |name: &str| REL.replace("\"draft\"", &format!("\"name\":\"{name}\",\"draft\""));
        let ver = |name: &str| parse_release(&with(name), "me/AutoKey", 1).unwrap().version;
        assert_eq!(ver("AutoKey 3.4.1 (build 58)").as_deref(), Some("3.4.1"));
        assert_eq!(ver("AutoKey build 58"), None);
        assert_eq!(ver("AutoKey 3.4.x (build 58)"), None);
        assert_eq!(ver("AutoKey 1.2.3.4.5.6.7.8.9.10"), None);
        assert_eq!(ver("Autre 3.4.1"), None);
    }

    #[test]
    fn notes_by_language() {
        let body = "- [fr] Un\\n- [en] One\\n- [pt] Um\\n\\nBuild n°1\\n- [de] Eins\\n- [fr]   \\n- [fr] Deux\\u0007";
        let json = REL.replace("\"draft\"", &format!("\"body\":\"{body}\",\"draft\""));
        let r = parse_release(&json, "me/AutoKey", 1).unwrap();
        let fr: Vec<_> = r.notes.iter().filter(|(c, _)| c == "fr").map(|(_, t)| t.as_str()).collect();
        assert_eq!(fr, ["Un", "Deux"], "ligne vide ignorée, contrôle supprimé");
        assert_eq!(r.notes.len(), 4, "langue inconnue ignorée");
        assert!(r.notes.contains(&("pt".to_string(), "Um".to_string())));
    }

    #[test]
    fn notes_limits() {
        let many: String = (0..9).map(|i| format!("- [en] n{i}\n")).collect();
        assert_eq!(parse_notes(&many).len(), 6);
        let long = format!("- [en] {}", "é".repeat(300));
        assert_eq!(parse_notes(&long)[0].1.chars().count(), 140);
    }
}
