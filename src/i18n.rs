use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Fr,
    En,
    Pt,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::Fr, Lang::En, Lang::Pt];

    pub fn code(self) -> &'static str {
        match self {
            Lang::Fr => "FR",
            Lang::En => "EN",
            Lang::Pt => "PT",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Lang::Fr => "Français",
            Lang::En => "English",
            Lang::Pt => "Português",
        }
    }

    fn idx(self) -> usize {
        self as usize
    }

    /// Séparateur décimal
    pub fn comma(self) -> bool {
        matches!(self, Lang::Fr | Lang::Pt)
    }
}

// clé, [fr, en, pt]
static T: &[(&str, [&str; 3])] = &[
    ("spam", ["Spam", "Spam", "Spam"]),
    ("pattern", ["Pattern", "Pattern", "Padrão"]),
    ("key", ["TOUCHE", "KEY", "TECLA"]),
    ("interval", ["INTERVALLE", "INTERVAL", "INTERVALO"]),
    ("hotkey", ["RACCOURCI", "HOTKEY", "ATALHO"]),
    ("every", ["Une pression toutes les", "One press every", "Um toque a cada"]),
    ("toggle", ["Bascule", "Toggle", "Alternar"]),
    ("hold", ["Maintien", "Hold", "Segurar"]),
    ("start", ["Démarrer", "Start", "Iniciar"]),
    ("stop", ["Arrêter", "Stop", "Parar"]),
    ("idle", ["Prêt", "Ready", "Pronto"]),
    ("click_change", ["Clique pour changer", "Click to change", "Clique para mudar"]),
    ("press_key", ["Appuie sur une touche…", "Press a key…", "Pressione uma tecla…"]),
    ("press_combo", ["Appuie sur ta touche ou ta combinaison…", "Press your key or combination…", "Pressione sua tecla ou combinação…"]),
    ("none", ["Aucun", "None", "Nenhum"]),
    (
        "hk_toggle_hint",
        ["Appuie une fois pour lancer, une fois pour arrêter", "Press once to start, once to stop", "Pressione uma vez para iniciar, outra para parar"],
    ),
    ("hk_hold_hint", ["Actif uniquement tant que tu maintiens", "Only active while you hold it", "Ativo só enquanto você segura"]),
    ("rate_s", ["≈ {n} pressions / seconde", "≈ {n} presses / second", "≈ {n} toques / segundo"]),
    ("rate_m", ["≈ {n} pressions / minute", "≈ {n} presses / minute", "≈ {n} toques / minuto"]),
    ("rate_h", ["≈ {n} pressions / heure", "≈ {n} presses / hour", "≈ {n} toques / hora"]),
    ("k_hold", ["Maintenir", "Hold", "Segurar"]),
    ("k_press", ["Appuyer", "Press", "Pressionar"]),
    ("k_wait", ["Attendre", "Wait", "Esperar"]),
    ("for", ["pendant", "for", "por"]),
    ("times", ["Nombre", "Count", "Vezes"]),
    ("gap", ["écart", "gap", "intervalo"]),
    ("loops", ["BOUCLES", "LOOPS", "REPETIÇÕES"]),
    ("infinite", ["Infini", "Infinite", "Infinito"]),
    ("add", ["Ajouter une étape", "Add a step", "Adicionar etapa"]),
    ("example", ["Exemple", "Example", "Exemplo"]),
    ("open", ["Ouvrir", "Open", "Abrir"]),
    ("save", ["Enregistrer", "Save", "Salvar"]),
    ("clear", ["Vider", "Clear", "Limpar"]),
    ("steps", ["ÉTAPES", "STEPS", "ETAPAS"]),
    ("cycle", ["Durée d'un cycle ≈ {d}", "One cycle ≈ {d}", "Um ciclo ≈ {d}"]),
    (
        "empty",
        [
            "Ajoute une étape à droite, ou charge l'exemple pour démarrer.",
            "Add a step on the right, or load the example to get started.",
            "Adicione uma etapa à direita ou carregue o exemplo para começar.",
        ],
    ),
    ("running_spam", ["En cours — {n} pressions", "Running — {n} presses", "Em execução — {n} toques"]),
    ("running_pat", ["En cours — boucle {i}/{n} · étape {s}/{t}", "Running — loop {i}/{n} · step {s}/{t}", "Em execução — repetição {i}/{n} · etapa {s}/{t}"]),
    ("hk_conflict", ["Conflit : touche déjà utilisée", "Conflict: key already in use", "Conflito: tecla já usada"]),
    ("macro_rec", ["Enregistrer", "Record", "Gravar"]),
    ("macro_stop", ["Arrêter · {n} évts · {t}", "Stop · {n} ev · {t}", "Parar · {n} ev · {t}"]),
    (
        "macro_tip",
        [
            "Enregistre vos frappes en étapes. Arrêt : ce bouton ou {k}",
            "Records your keystrokes as steps. Stop: this button or {k}",
            "Grava suas teclas como etapas. Parar: este botão ou {k}",
        ],
    ),
    ("macro_msg", ["Enregistrement de macro… {n} évts", "Recording macro… {n} ev", "Gravando macro… {n} ev"]),
    ("macro_empty", ["Rien enregistré.", "Nothing recorded.", "Nada gravado."]),
    ("macro_trunc", ["Macro tronquée à 500 étapes.", "Macro truncated to 500 steps.", "Macro truncada em 500 etapas."]),
    ("macro_busy", ["Arrêtez d'abord la tâche en cours.", "Stop the running task first.", "Pare primeiro a tarefa em execução."]),
    ("macro_replace_q", ["Remplacer les étapes actuelles ?", "Replace the current steps?", "Substituir as etapas atuais?"]),
    ("no_steps", ["Ajoute au moins une étape.", "Add at least one step.", "Adicione pelo menos uma etapa."]),
    ("bad_file", ["Fichier invalide.", "Invalid file.", "Arquivo inválido."]),
    ("saved", ["Pattern enregistré.", "Pattern saved.", "Padrão salvo."]),
    ("pin", ["Toujours au premier plan", "Always on top", "Sempre no topo"]),
    ("minimize", ["Réduire", "Minimize", "Minimizar"]),
    ("close", ["Fermer", "Close", "Fechar"]),
    ("settings", ["Réglages", "Settings", "Configurações"]),
    ("jitter", ["Aléa sur les délais", "Random delay variation", "Variação aleatória"]),
    ("beep", ["Bip au démarrage / arrêt", "Beep on start / stop", "Bip ao iniciar / parar"]),
    ("profiles", ["Profils", "Profiles", "Perfis"]),
    ("prof_new", ["Nouveau profil", "New profile", "Novo perfil"]),
    ("prof_default", ["Profil {n}", "Profile {n}", "Perfil {n}"]),
    ("prof_none", ["Aucun profil enregistré", "No saved profiles", "Nenhum perfil salvo"]),
    ("prof_overwrite", ["Écraser avec la configuration actuelle", "Overwrite with current setup", "Substituir pela configuração atual"]),
    ("prof_rename", ["Renommer", "Rename", "Renomear"]),
    ("prof_delete", ["Supprimer le profil", "Delete profile", "Excluir perfil"]),
    ("prof_load", ["Charger ce profil", "Load this profile", "Carregar este perfil"]),
    ("prof_del_q", ["Supprimer le profil « {name} » ?", "Delete profile \"{name}\"?", "Excluir o perfil \"{name}\"?"]),
    ("prof_full", ["Maximum de 20 profils atteint", "Maximum of 20 profiles reached", "Máximo de 20 perfis atingido"]),
    (
        "prof_too_big",
        [
            "Pattern trop long pour un profil (100 étapes max)",
            "Pattern too long for a profile (100 steps max)",
            "Padrão longo demais para um perfil (máx. 100 etapas)",
        ],
    ),
    ("on", ["Oui", "On", "Sim"]),
    ("off", ["Non", "Off", "Não"]),
    ("limit", ["Arrêt automatique", "Auto stop", "Parada automática"]),
    ("limit_none", ["Aucun", "None", "Nenhum"]),
    ("limit_count", ["Pressions", "Presses", "Toques"]),
    ("limit_dur", ["Durée", "Time", "Tempo"]),
    ("press_dur", ["Durée d'appui", "Key press duration", "Duração do toque"]),
    ("stop_hk", ["Arrêt d'urgence", "Emergency stop", "Parada de emergência"]),
    ("language", ["Langue", "Language", "Idioma"]),
    ("left", ["Déplacer à gauche", "Move left", "Mover para a esquerda"]),
    ("right", ["Déplacer à droite", "Move right", "Mover para a direita"]),
    ("dup", ["Dupliquer", "Duplicate", "Duplicar"]),
    ("del", ["Supprimer", "Delete", "Excluir"]),
    ("clear_q", ["Supprimer toutes les étapes ?", "Delete all steps?", "Excluir todas as etapas?"]),
    ("yes", ["Supprimer", "Delete", "Excluir"]),
    ("cancel", ["Annuler", "Cancel", "Cancelar"]),
    (
        "bg_hint",
        [
            "Les raccourcis marchent même quand la fenêtre est en arrière-plan",
            "Hotkeys work even when the window is in the background",
            "Os atalhos funcionam mesmo com a janela em segundo plano",
        ],
    ),
    ("update", ["Mettre à jour", "Update", "Atualizar"]),
    (
        "update_tip",
        [
            "Nouvelle version disponible (build {b}) — clique pour l'installer",
            "New version available (build {b}) — click to install",
            "Nova versão disponível (build {b}) — clique para instalar",
        ],
    ),
    ("up_dl", ["Téléchargement de la mise à jour… {p} %", "Downloading update… {p}%", "Baixando atualização… {p}%"]),
    ("up_restart", ["Mise à jour installée, redémarrage…", "Update installed, restarting…", "Atualização instalada, reiniciando…"]),
    ("up_net", ["Mise à jour impossible : connexion échouée", "Update failed: connection error", "Falha na atualização: erro de conexão"]),
    ("up_bad", ["Mise à jour refusée : fichier non vérifié", "Update rejected: file failed verification", "Atualização recusada: arquivo não verificado"]),
    ("up_io", ["Mise à jour impossible : dossier protégé en écriture", "Update failed: folder is write-protected", "Falha na atualização: pasta protegida"]),
    // noms de touches
    ("Space", ["Espace", "Space", "Espaço"]),
    ("Enter", ["Entrée", "Enter", "Enter"]),
    ("Esc", ["Échap", "Esc", "Esc"]),
    ("Del", ["Suppr", "Del", "Del"]),
    ("Back", ["Retour", "Backspace", "Backspace"]),
    ("Shift", ["Maj", "Shift", "Shift"]),
    ("LShift", ["Maj G", "L-Shift", "Shift E"]),
    ("RShift", ["Maj D", "R-Shift", "Shift D"]),
    ("Ctrl", ["Ctrl", "Ctrl", "Ctrl"]),
    ("LCtrl", ["Ctrl G", "L-Ctrl", "Ctrl E"]),
    ("RCtrl", ["Ctrl D", "R-Ctrl", "Ctrl D"]),
    ("Caps", ["Verr. Maj", "Caps Lock", "Caps Lock"]),
    ("Ins", ["Inser", "Ins", "Ins"]),
    ("Home", ["Début", "Home", "Home"]),
    ("End", ["Fin", "End", "End"]),
    ("PgUp", ["Pg Préc", "PgUp", "PgUp"]),
    ("PgDn", ["Pg Suiv", "PgDn", "PgDn"]),
];

pub fn tr(lang: Lang, key: &str) -> &'static str {
    for (k, v) in T {
        if *k == key {
            return v[lang.idx()];
        }
    }
    // clé inconnue : on renvoie une chaîne statique vide plutôt que de paniquer
    ""
}

/// tr + remplacement des {x}
pub fn trf(lang: Lang, key: &str, args: &[(&str, String)]) -> String {
    let mut s = tr(lang, key).to_string();
    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

pub fn fmt_num(lang: Lang, v: f64) -> String {
    let mut s = format!("{v:.3}");
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if lang.comma() {
        s = s.replace('.', ",");
    }
    s
}

pub fn fmt_duration(lang: Lang, sec: f64) -> String {
    if sec < 1.0 {
        format!("{} ms", fmt_num(lang, (sec * 1000.0).round()))
    } else if sec < 60.0 {
        format!("{} s", fmt_num(lang, (sec * 100.0).round() / 100.0))
    } else {
        let m = (sec / 60.0).floor();
        let s = sec - m * 60.0;
        if s >= 0.05 {
            format!("{} min {} s", m as u64, fmt_num(lang, (s * 10.0).round() / 10.0))
        } else {
            format!("{} min", m as u64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_unique_and_translations_non_empty() {
        let mut seen = std::collections::HashSet::new();
        for (k, v) in T {
            assert!(seen.insert(*k), "clé en double : {k}");
            for (i, s) in v.iter().enumerate() {
                assert!(!s.trim().is_empty(), "traduction vide : {k} (langue {i})");
            }
        }
    }

    /// Littéraux passés en 2e argument de `tr(` / `trf(` dans un source
    fn used_keys(src: &str) -> Vec<String> {
        let mut out = Vec::new();
        for name in ["tr(", "trf("] {
            for (pos, _) in src.match_indices(name) {
                // exclut les identifiants plus longs (ex. `str(`, `entry(`)
                if src[..pos].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                    continue;
                }
                let rest = &src[pos + name.len()..];
                let Some(comma) = rest.find(',') else { continue };
                let Some(lit) = rest[comma + 1..].trim_start().strip_prefix('"') else { continue };
                if let Some(end) = lit.find('"') {
                    out.push(lit[..end].to_string());
                }
            }
        }
        out
    }

    #[test]
    fn used_keys_parser() {
        let k = used_keys(r#"x = tr(lang, "a"); trf(self.lang, "b", &[]); str(lang, "no"); tr(lang, k);"#);
        assert_eq!(k, vec!["a", "b"]);
    }

    #[test]
    fn literal_keys_exist() {
        let files = [
            ("ui/mod.rs", include_str!("ui/mod.rs")),
            ("ui/chrome.rs", include_str!("ui/chrome.rs")),
            ("ui/spam.rs", include_str!("ui/spam.rs")),
            ("ui/pattern.rs", include_str!("ui/pattern.rs")),
            ("ui/widgets.rs", include_str!("ui/widgets.rs")),
            ("keys.rs", include_str!("keys.rs")),
        ];
        let mut total = 0;
        for (name, src) in files {
            for k in used_keys(src) {
                total += 1;
                assert!(T.iter().any(|(t, _)| *t == k), "clé inconnue dans {name} : {k}");
            }
        }
        assert!(total > 20, "extraction suspecte : {total} clés");
    }
}
