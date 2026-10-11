//! Mise à jour automatique depuis les Releases GitHub du dépôt qui a compilé l'exe.
//! Sécurité : HTTPS uniquement (rustls), seules les URL du dépôt sont acceptées,
//! taille limitée, empreinte SHA-256 vérifiée, signature « MZ » vérifiée,
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
const MAX_EXE: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum UpState {
    Idle,
    Available { build: u64, exe_url: String, sha_url: String },
    Downloading(u8),
    Failed(&'static str),
    Restarting,
}

pub struct Updater {
    pub state: Mutex<UpState>,
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

/// Analyse la réponse JSON de /releases/latest. Renvoie (build, url exe, url sha) si plus récent.
pub fn parse_release(json: &str, repo: &str, cur: u64) -> Option<(u64, String, String)> {
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
    for a in v.get("assets")?.as_array()? {
        let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let url = a.get("browser_download_url").and_then(|n| n.as_str()).unwrap_or("");
        if !url.starts_with(&prefix) {
            continue; // on n'accepte que les fichiers de CE dépôt
        }
        match name {
            EXE_ASSET => exe = Some(url.to_string()),
            SHA_ASSET => sha = Some(url.to_string()),
            _ => {}
        }
    }
    Some((build, exe?, sha?))
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
        Arc::new(Updater { state: Mutex::new(UpState::Idle), quit: AtomicBool::new(false), sh })
    }

    pub fn state(&self) -> UpState {
        self.state.lock().unwrap().clone()
    }

    fn set(&self, s: UpState) {
        *self.state.lock().unwrap() = s;
        self.sh.repaint();
    }

    /// Vérifie au démarrage puis toutes les 6 h ; nettoie l'ancien exe après une mise à jour.
    pub fn spawn_checker(self: &Arc<Self>) {
        let me = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(4));
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::fs::remove_file(old_path(&exe));
            }
            let (Some(repo), true) = (REPO, current_build() > 0) else { return };
            loop {
                if matches!(me.state(), UpState::Idle | UpState::Failed(_)) {
                    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
                    let resp = agent().get(&url).set("Accept", "application/vnd.github+json").call();
                    if let Ok(r) = resp {
                        if let Ok(body) = r.into_string() {
                            if let Some((build, exe_url, sha_url)) = parse_release(&body, repo, current_build()) {
                                me.set(UpState::Available { build, exe_url, sha_url });
                            }
                        }
                    }
                }
                std::thread::sleep(Duration::from_secs(6 * 3600));
            }
        });
    }

    pub fn install(self: &Arc<Self>) {
        let UpState::Available { exe_url, sha_url, .. } = self.state() else { return };
        self.set(UpState::Downloading(0));
        let me = self.clone();
        std::thread::spawn(move || {
            if let Err(e) = me.do_install(&exe_url, &sha_url) {
                me.set(UpState::Failed(e));
            }
        });
    }

    fn do_install(&self, exe_url: &str, sha_url: &str) -> Result<(), &'static str> {
        let ag = agent();
        // 1) empreinte attendue
        let sha_txt = ag.get(sha_url).call().map_err(|_| "up_net")?.into_string().map_err(|_| "up_net")?;
        let expected = parse_sha(&sha_txt).ok_or("up_bad")?;
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
            if total > 0 {
                self.set(UpState::Downloading(((data.len() as u64 * 100) / total).min(99) as u8));
            }
        }
        // 3) vérifications
        let got: [u8; 32] = hasher.finalize().into();
        verify(&data, &got, &expected)?;
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
        {"name":"AutoKey.exe.sha256","browser_download_url":"https://github.com/me/AutoKey/releases/download/build-57/AutoKey.exe.sha256"}]}"#;

    #[test]
    fn newer_release_detected() {
        let r = parse_release(REL, "me/AutoKey", 42).unwrap();
        assert_eq!(r.0, 57);
        assert!(r.1.ends_with("/AutoKey.exe"));
        assert!(parse_release(REL, "me/AutoKey", 57).is_none(), "même build : rien");
    }

    #[test]
    fn foreign_urls_rejected() {
        assert!(parse_release(REL, "pirate/AutoKey", 1).is_none());
        let evil = REL.replace("https://github.com/me", "https://evil.example/me");
        assert!(parse_release(&evil, "me/AutoKey", 1).is_none());
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
}
