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
    ("compact", ["Vue réduite", "Compact view", "Vista reduzida"]),
    ("compact_back", ["Vue complète", "Full view", "Vista completa"]),
    ("tray", ["Zone de notification", "Tray icon", "Ícone na bandeja"]),
    (
        "tray_tip",
        [
            "Fermer / réduire cache la fenêtre dans la zone de notification",
            "Close / minimize hides the window in the notification area",
            "Fechar / minimizar oculta a janela na área de notificação",
        ],
    ),
    ("tray_show", ["Afficher", "Show", "Mostrar"]),
    ("tray_start_spam", ["Démarrer le spam", "Start spam", "Iniciar spam"]),
    ("tray_stop_spam", ["Arrêter le spam", "Stop spam", "Parar spam"]),
    ("tray_start_pat", ["Démarrer le pattern", "Start pattern", "Iniciar padrão"]),
    ("tray_stop_pat", ["Arrêter le pattern", "Stop pattern", "Parar padrão"]),
    ("tray_quit", ["Quitter", "Quit", "Sair"]),
    ("tray_running", ["En cours", "Running", "Em execução"]),
    ("jitter", ["Aléa sur les délais", "Random delay variation", "Variação aleatória"]),
    ("bind_win", ["Lier à la fenêtre cible", "Bind to target window", "Vincular à janela alvo"]),
    ("paused", ["En pause — fenêtre cible inactive", "Paused — target window inactive", "Em pausa — janela alvo inativa"]),
    ("arming", ["En attente de la fenêtre cible…", "Waiting for the target window…", "Aguardando a janela alvo…"]),
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
    ("upd_title", ["Mise à jour disponible", "Update available", "Atualização disponível"]),
    ("upd_news", ["NOUVEAUTÉS", "WHAT'S NEW", "NOVIDADES"]),
    ("upd_later", ["Plus tard", "Later", "Mais tarde"]),
    ("up_dl", ["Téléchargement de la mise à jour… {p} %", "Downloading update… {p}%", "Baixando atualização… {p}%"]),
    ("up_restart", ["Mise à jour installée, redémarrage…", "Update installed, restarting…", "Atualização instalada, reiniciando…"]),
    ("up_net", ["Mise à jour impossible : connexion échouée", "Update failed: connection error", "Falha na atualização: erro de conexão"]),
    ("up_bad", ["Mise à jour refusée : fichier non vérifié", "Update rejected: file failed verification", "Atualização recusada: arquivo não verificado"]),
    ("up_io", ["Mise à jour impossible : dossier protégé en écriture", "Update failed: folder is write-protected", "Falha na atualização: pasta protegida"]),
    // noms de touches
    ("Space", ["Espace", "Space", "Espaço"]),
    ("sec_exec", ["EXÉCUTION", "EXECUTION", "EXECUÇÃO"]),
    ("sec_ui", ["INTERFACE", "INTERFACE", "INTERFACE"]),
    ("sec_sys", ["SYSTÈME", "SYSTEM", "SISTEMA"]),
    ("sec_upd", ["MISES À JOUR", "UPDATES", "ATUALIZAÇÕES"]),
    ("sec_data", ["DONNÉES", "DATA", "DADOS"]),
    ("start_min", ["Démarrer caché", "Start hidden", "Iniciar oculto"]),
    ("autostart", ["Lancer avec Windows", "Launch with Windows", "Iniciar com o Windows"]),
    ("autostart_fix", ["Mettre à jour le chemin", "Update path", "Atualizar caminho"]),
    ("autostart_err", ["Registre inaccessible.", "Registry unavailable.", "Registro inacessível."]),
    ("auto_upd", ["Vérifier au démarrage", "Check at startup", "Verificar ao iniciar"]),
    ("version", ["Version", "Version", "Versão"]),
    ("check_now", ["Vérifier maintenant", "Check now", "Verificar agora"]),
    ("chk_busy", ["Vérification…", "Checking…", "Verificando…"]),
    ("chk_ok", ["AutoKey est à jour.", "AutoKey is up to date.", "O AutoKey está atualizado."]),
    ("chk_avail", ["Build {b} disponible : bouton « Mettre à jour » dans la barre.", "Build {b} available: use the “Update” button in the title bar.", "Build {b} disponível: use o botão “Atualizar” na barra."]),
    ("chk_err", ["Vérification impossible (connexion).", "Check failed (connection).", "Falha na verificação (conexão)."]),
    ("chk_na", ["Indisponible (build local).", "Unavailable (local build).", "Indisponível (build local)."]),
    ("d_folder", ["Dossier de configuration", "Configuration folder", "Pasta de configuração"]),
    ("d_open", ["Ouvrir", "Open", "Abrir"]),
    ("d_export", ["Exporter la configuration", "Export configuration", "Exportar configuração"]),
    ("d_exp_btn", ["Exporter", "Export", "Exportar"]),
    ("d_import", ["Importer une configuration", "Import configuration", "Importar configuração"]),
    ("d_imp_btn", ["Importer", "Import", "Importar"]),
    ("d_reset", ["Réinitialiser les réglages", "Reset settings", "Redefinir configurações"]),
    ("d_reset_btn", ["Réinitialiser", "Reset", "Redefinir"]),
    ("io_err", ["Écriture impossible.", "Could not write the file.", "Não foi possível gravar."]),
    ("cfg_exported", ["Configuration exportée.", "Configuration exported.", "Configuração exportada."]),
    ("cfg_imported", ["Configuration importée.", "Configuration imported.", "Configuração importada."]),
    ("cfg_reset", ["Réglages réinitialisés.", "Settings reset.", "Configurações redefinidas."]),
    ("reset_q", ["Réinitialiser tous les réglages ?", "Reset all settings?", "Redefinir todas as configurações?"]),
    ("reset_note", ["Les profils sont conservés.", "Profiles are kept.", "Os perfis são mantidos."]),
    ("reset_yes", ["Réinitialiser", "Reset", "Redefinir"]),
    ("tip_press", ["Durée pendant laquelle chaque touche reste enfoncée (0 à 500 ms). Certains jeux ignorent les appuis trop brefs. En spam, limitée à la moitié de l'intervalle.", "How long each key stays pressed (0 to 500 ms). Some games ignore very short presses. In spam mode it is capped at half the interval.", "Tempo em que cada tecla fica pressionada (0 a 500 ms). Alguns jogos ignoram toques muito curtos. No spam, limitado à metade do intervalo."]),
    ("tip_jitter", ["Fait varier chaque délai au hasard de ± ce pourcentage (0 à 50 %) pour un rythme moins régulier.", "Randomly varies each delay by ± this percentage (0 to 50%) for a less regular rhythm.", "Varia cada atraso aleatoriamente em ± esta porcentagem (0 a 50%) para um ritmo menos regular."]),
    ("tip_bind", ["N'envoie les touches que si la fenêtre au premier plan est celle où la tâche a été lancée ; sinon la tâche se met en pause.", "Keys are only sent while the window that was in front when the task started is still in front; otherwise the task pauses.", "As teclas só são enviadas se a janela em primeiro plano for a mesma em que a tarefa foi iniciada; caso contrário, a tarefa pausa."]),
    ("tip_stop", ["Raccourci global qui arrête immédiatement toute tâche, quel que soit le mode. Cliquez puis appuyez sur la combinaison voulue.", "Global shortcut that immediately stops any running task, whatever the mode. Click, then press the combination you want.", "Atalho global que interrompe imediatamente qualquer tarefa, em qualquer modo. Clique e pressione a combinação desejada."]),
    ("tip_lang", ["Langue de l'interface (FR, EN ou PT).", "Interface language (FR, EN or PT).", "Idioma da interface (FR, EN ou PT)."]),
    ("tip_pin", ["Garde la fenêtre d'AutoKey au-dessus des autres fenêtres.", "Keeps the AutoKey window above other windows.", "Mantém a janela do AutoKey acima das outras janelas."]),
    ("tip_compact", ["Petite fenêtre avec seulement le mode, le raccourci et le bouton démarrer / arrêter.", "Small window with only the mode, the shortcut and the start / stop button.", "Janela pequena apenas com o modo, o atalho e o botão iniciar / parar."]),
    ("tip_beep", ["Émet un court bip au démarrage et à l'arrêt de la tâche, utile quand la fenêtre est cachée.", "Plays a short beep when a task starts and stops, handy when the window is hidden.", "Emite um bip curto ao iniciar e parar a tarefa, útil quando a janela está oculta."]),
    ("tip_tray", ["Affiche une icône près de l'horloge. Fermer ou réduire cache alors la fenêtre au lieu de quitter ; « Quitter » est dans le menu clic droit de l'icône.", "Shows an icon next to the clock. Closing or minimizing then hides the window instead of quitting; “Quit” is in the icon's right-click menu.", "Mostra um ícone perto do relógio. Fechar ou minimizar oculta a janela em vez de sair; “Sair” fica no menu de clique direito do ícone."]),
    ("tip_startmin", ["Au lancement, la fenêtre reste cachée dans la zone de notification. Sans effet si l'icône est désactivée.", "On launch, the window stays hidden in the notification area. Has no effect if the tray icon is off.", "Ao iniciar, a janela fica oculta na área de notificação. Sem efeito se o ícone estiver desativado."]),
    ("tip_autostart", ["Lance AutoKey à l'ouverture de votre session Windows (clé utilisateur du registre, sans droits administrateur). L'état affiché est lu dans le registre.", "Launches AutoKey when you sign in to Windows (per-user registry key, no administrator rights). The state shown is read from the registry.", "Inicia o AutoKey ao entrar no Windows (chave de usuário do registro, sem direitos de administrador). O estado exibido é lido do registro."]),
    ("tip_version", ["Version et numéro de build de cet exécutable.", "Version and build number of this executable.", "Versão e número de build deste executável."]),
    ("tip_autoupd", ["Cherche une nouvelle version sur GitHub au démarrage puis toutes les 6 h (pris en compte au prochain lancement). Les mises à jour restent vérifiées (SHA-256 + signature).", "Looks for a new version on GitHub at startup then every 6 h (applied at next launch). Updates are still verified (SHA-256 + signature).", "Procura uma nova versão no GitHub ao iniciar e a cada 6 h (vale na próxima inicialização). As atualizações continuam verificadas (SHA-256 + assinatura)."]),
    ("tip_check", ["Interroge GitHub maintenant. Si une version plus récente existe, le bouton « Mettre à jour » apparaît dans la barre de titre.", "Asks GitHub right now. If a newer version exists, the “Update” button appears in the title bar.", "Consulta o GitHub agora. Se houver uma versão mais recente, o botão “Atualizar” aparece na barra de título."]),
    ("tip_folder", ["Ouvre dans l'explorateur le dossier qui contient le fichier de configuration (config3.json).", "Opens the folder that holds the configuration file (config3.json) in Explorer.", "Abre no Explorador a pasta que contém o arquivo de configuração (config3.json)."]),
    ("tip_export", ["Enregistre tous vos réglages (raccourcis, spam, pattern, profils…) dans un fichier .json.", "Saves all your settings (shortcuts, spam, pattern, profiles…) to a .json file.", "Salva todas as suas configurações (atalhos, spam, padrão, perfis…) em um arquivo .json."]),
    ("tip_import", ["Remplace tous vos réglages par ceux d'un fichier exporté. Refusé si le fichier est invalide, dépasse 1 Mo ou contient des raccourcis en conflit.", "Replaces all your settings with those of an exported file. Rejected if the file is invalid, over 1 MB or has conflicting shortcuts.", "Substitui todas as suas configurações pelas de um arquivo exportado. Recusado se o arquivo for inválido, maior que 1 MB ou tiver atalhos em conflito."]),
    ("tip_reset", ["Remet tous les réglages à leur valeur par défaut. Vos profils sont conservés.", "Puts every setting back to its default. Your profiles are kept.", "Restaura todas as configurações para o padrão. Seus perfis são mantidos."]),
    ("tip_spam_key", ["La touche envoyée en boucle. Cliquez sur la grande touche, puis appuyez sur celle à spammer.", "The key sent over and over. Click the big key, then press the one to spam.", "A tecla enviada repetidamente. Clique na tecla grande e pressione a que quer spammar."]),
    ("tip_interval", ["Temps entre deux appuis (minimum 10 ms). Les pastilles donnent des valeurs courantes.", "Time between two presses (minimum 10 ms). The chips give common values.", "Tempo entre dois toques (mínimo 10 ms). Os atalhos dão valores comuns."]),
    ("tip_limit", ["Arrête le spam tout seul : Aucun = jusqu'à l'arrêt manuel, Pressions = après N appuis, Durée = après N secondes.", "Stops the spam by itself: None = until you stop it, Presses = after N presses, Duration = after N seconds.", "Para o spam sozinho: Nenhum = até você parar, Toques = após N toques, Duração = após N segundos."]),
    ("tip_hotkey", ["Combinaison globale qui démarre/arrête la tâche. Bascule : un appui démarre, un autre arrête. Maintien : actif tant que vous la tenez.", "Global combination that starts/stops the task. Toggle: one press starts, another stops. Hold: active while you hold it.", "Combinação global que inicia/para a tarefa. Alternar: um toque inicia, outro para. Manter: ativo enquanto você segura."]),
    ("tip_loops", ["Nombre de fois que la séquence est rejouée. Le bouton ∞ la répète jusqu'à l'arrêt.", "Number of times the sequence is replayed. The ∞ button repeats it until stopped.", "Quantas vezes a sequência é repetida. O botão ∞ repete até você parar."]),
    ("tip_kinds", ["Maintenir : garde la touche enfoncée un temps donné. Appuyer : un appui bref. Attendre : une pause sans touche.", "Hold: keeps the key down for a set time. Press: a quick tap. Wait: a pause with no key.", "Segurar: segura a tecla por um tempo. Pressionar: um toque rápido. Esperar: uma pausa sem tecla."]),
    ("tip_pat_tools", ["Enregistrer (rouge) : capture vos touches en direct et les convertit en étapes Maintenir / Appuyer / Attendre. Les touches de raccourci sont ignorées ; arrêtez par un nouveau clic ou le raccourci d'arrêt d'urgence.\nOuvrir / Enregistrer (disquette) : importe ou exporte le pattern dans un fichier .json.", "Record (red): captures your keys live and converts them into Hold / Press / Wait steps. Shortcut keys are ignored; stop with another click or the emergency stop shortcut.\nOpen / Save (disk): imports or exports the pattern as a .json file.", "Gravar (vermelho): captura suas teclas ao vivo e as converte em etapas Segurar / Pressionar / Esperar. Teclas de atalho são ignoradas; pare com outro clique ou com o atalho de parada de emergência.\nAbrir / Salvar (disquete): importa ou exporta o padrão em um arquivo .json."]),
    ("tip_profiles", ["Un profil mémorise un spam et un pattern (100 étapes max). Les raccourcis globaux ne sont pas chargés avec lui.", "A profile stores a spam and a pattern (100 steps max). Global shortcuts are not loaded with it.", "Um perfil guarda um spam e um padrão (máx. 100 etapas). Os atalhos globais não são carregados com ele."]),
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
            ("ui/settings.rs", include_str!("ui/settings.rs")),
            ("keys.rs", include_str!("keys.rs")),
            ("platform.rs", include_str!("platform.rs")),
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

    #[test]
    fn tip_keys_complete_used_and_short() {
        let src = [include_str!("ui/spam.rs"), include_str!("ui/pattern.rs"), include_str!("ui/chrome.rs"), include_str!("ui/settings.rs")].concat();
        let used = used_keys(&src);
        let tips: Vec<_> = T.iter().filter(|(k, _)| k.starts_with("tip_")).collect();
        assert!(tips.len() >= 25, "info-bulles manquantes : {}", tips.len());
        for (k, v) in tips {
            assert!(used.iter().any(|u| u == k), "info-bulle jamais affichée : {k}");
            for (i, s) in v.iter().enumerate() {
                assert!(s.trim().len() >= 20 && s.len() <= 420, "{k} (langue {i}) : longueur {}", s.len());
            }
        }
        // aucune valeur vide, quelle que soit la langue
        assert!(T.iter().all(|(_, v)| v.iter().all(|s| !s.is_empty())));
    }
}
