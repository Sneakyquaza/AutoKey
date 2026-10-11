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
    (
        "press_combo",
        ["Appuie sur ta touche ou ta combinaison…", "Press your key or combination…", "Pressione sua tecla ou combinação…"],
    ),
    ("none", ["Aucun", "None", "Nenhum"]),
    (
        "hk_toggle_hint",
        ["Appuie une fois pour lancer, une fois pour arrêter", "Press once to start, once to stop", "Pressione uma vez para iniciar, outra para parar"],
    ),
    (
        "hk_hold_hint",
        ["Actif uniquement tant que tu maintiens", "Only active while you hold it", "Ativo só enquanto você segura"],
    ),
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
    (
        "running_pat",
        ["En cours — boucle {i}/{n} · étape {s}/{t}", "Running — loop {i}/{n} · step {s}/{t}", "Em execução — repetição {i}/{n} · etapa {s}/{t}"],
    ),
    ("no_steps", ["Ajoute au moins une étape.", "Add at least one step.", "Adicione pelo menos uma etapa."]),
    ("bad_file", ["Fichier invalide.", "Invalid file.", "Arquivo inválido."]),
    ("saved", ["Pattern enregistré.", "Pattern saved.", "Padrão salvo."]),
    ("pin", ["Toujours au premier plan", "Always on top", "Sempre no topo"]),
    ("minimize", ["Réduire", "Minimize", "Minimizar"]),
    ("close", ["Fermer", "Close", "Fechar"]),
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
