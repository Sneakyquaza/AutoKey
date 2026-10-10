"""
AutoKey 2 — auto-presser clavier : spam d'une touche et patterns en boucle.
Dépendances : pip install pynput pyside6
"""
import json
import os
import sys
import threading

from PySide6.QtCore import Qt, QObject, Signal, QTimer, QPointF, QRectF, QSize
from PySide6.QtGui import (QColor, QPainter, QPen, QLinearGradient, QIcon,
                           QPixmap, QPainterPath, QFont)
from PySide6.QtWidgets import (QApplication, QWidget, QFrame, QLabel, QPushButton, QHBoxLayout,
                               QVBoxLayout, QStackedWidget, QScrollArea,
                               QLineEdit, QButtonGroup, QMenu, QFileDialog, QMessageBox,
                               QSizePolicy)

from pynput import keyboard
from pynput.keyboard import Key, KeyCode, Controller

APP_NAME = "AutoKey"
VERSION = "2.0"
IS_WIN = sys.platform == "win32"
UNITS = {"ms": 0.001, "s": 1.0, "min": 60.0}
kb = Controller()

C = {  # palette
    "bg": "#0B0D12", "surface": "#12151C", "surface2": "#181C25", "line": "#222838",
    "line2": "#2A3042", "text": "#E8EAF2", "muted": "#8A90A6",
    "violet": "#8B5CF6", "cyan": "#22D3EE", "amber": "#F59E0B",
    "rose": "#F43F5E", "orange": "#FB923C", "green": "#34D399",
}
KIND_COLOR = {"hold": C["violet"], "press": C["cyan"], "wait": C["amber"]}

# ------------------------------------------------------------------ i18n
LANGS = {"fr": "Français", "en": "English", "pt": "Português"}
T = {
    "fr": {
        "spam": "Spam", "pattern": "Pattern", "key": "TOUCHE", "interval": "INTERVALLE",
        "hotkey": "RACCOURCI", "every": "Une pression toutes les", "toggle": "Bascule",
        "hold": "Maintien", "start": "Démarrer", "stop": "Arrêter", "idle": "Prêt",
        "click_change": "Clique pour changer", "press_key": "Appuie sur une touche…",
        "press_combo": "Appuie sur ta touche ou ta combinaison…", "none": "Aucun",
        "hk_toggle_hint": "Appuie une fois pour lancer, une fois pour arrêter",
        "hk_hold_hint": "Actif uniquement tant que tu maintiens",
        "rate_s": "≈ {n} pressions / seconde", "rate_m": "≈ {n} pressions / minute",
        "rate_h": "≈ {n} pressions / heure",
        "k_hold": "Maintenir", "k_press": "Appuyer", "k_wait": "Attendre",
        "for": "pendant", "times": "fois", "gap": "écart", "loops": "BOUCLES",
        "infinite": "Infini", "add": "Ajouter une étape", "example": "Exemple",
        "open": "Ouvrir", "save": "Enregistrer", "clear": "Vider", "steps": "ÉTAPES", "bg_hint": "Les raccourcis marchent même quand la fenêtre est en arrière-plan",
        "cycle": "Durée d'un cycle ≈ {d}", "empty": "Ajoute une étape à droite, ou charge l'exemple pour démarrer.",
        "running_spam": "En cours — {n} pressions", "running_pat": "En cours — boucle {i}/{n} · étape {s}/{t}",
        "no_steps": "Ajoute au moins une étape.", "bad_file": "Fichier invalide.",
        "pin": "Toujours au premier plan", "minimize": "Réduire", "close": "Fermer",
        "language": "Langue", "left": "Déplacer à gauche", "right": "Déplacer à droite",
        "dup": "Dupliquer", "del": "Supprimer", "clear_q": "Supprimer toutes les étapes ?",
        "Space": "Espace", "Enter": "Entrée", "Esc": "Échap", "Del": "Suppr",
        "LCtrl": "Ctrl G", "RCtrl": "Ctrl D", "RShift": "Maj D", "Shift": "Maj",
    },
    "en": {
        "spam": "Spam", "pattern": "Pattern", "key": "KEY", "interval": "INTERVAL",
        "hotkey": "HOTKEY", "every": "One press every", "toggle": "Toggle",
        "hold": "Hold", "start": "Start", "stop": "Stop", "idle": "Ready",
        "click_change": "Click to change", "press_key": "Press a key…",
        "press_combo": "Press your key or combination…", "none": "None",
        "hk_toggle_hint": "Press once to start, once to stop",
        "hk_hold_hint": "Only active while you hold it",
        "rate_s": "≈ {n} presses / second", "rate_m": "≈ {n} presses / minute",
        "rate_h": "≈ {n} presses / hour",
        "k_hold": "Hold", "k_press": "Press", "k_wait": "Wait",
        "for": "for", "times": "times", "gap": "gap", "loops": "LOOPS",
        "infinite": "Infinite", "add": "Add a step", "example": "Example",
        "open": "Open", "save": "Save", "clear": "Clear", "steps": "STEPS", "bg_hint": "Hotkeys work even when the window is in the background",
        "cycle": "One cycle ≈ {d}", "empty": "Add a step on the right, or load the example to get started.",
        "running_spam": "Running — {n} presses", "running_pat": "Running — loop {i}/{n} · step {s}/{t}",
        "no_steps": "Add at least one step.", "bad_file": "Invalid file.",
        "pin": "Always on top", "minimize": "Minimize", "close": "Close",
        "language": "Language", "left": "Move left", "right": "Move right",
        "dup": "Duplicate", "del": "Delete", "clear_q": "Delete all steps?",
        "Space": "Space", "Enter": "Enter", "Esc": "Esc", "Del": "Del",
        "LCtrl": "L-Ctrl", "RCtrl": "R-Ctrl", "RShift": "R-Shift", "Shift": "Shift",
    },
    "pt": {
        "spam": "Spam", "pattern": "Padrão", "key": "TECLA", "interval": "INTERVALO",
        "hotkey": "ATALHO", "every": "Um toque a cada", "toggle": "Alternar",
        "hold": "Segurar", "start": "Iniciar", "stop": "Parar", "idle": "Pronto",
        "click_change": "Clique para mudar", "press_key": "Pressione uma tecla…",
        "press_combo": "Pressione sua tecla ou combinação…", "none": "Nenhum",
        "hk_toggle_hint": "Pressione uma vez para iniciar, outra para parar",
        "hk_hold_hint": "Ativo só enquanto você segura",
        "rate_s": "≈ {n} toques / segundo", "rate_m": "≈ {n} toques / minuto",
        "rate_h": "≈ {n} toques / hora",
        "k_hold": "Segurar", "k_press": "Pressionar", "k_wait": "Esperar",
        "for": "por", "times": "vezes", "gap": "intervalo", "loops": "REPETIÇÕES",
        "infinite": "Infinito", "add": "Adicionar etapa", "example": "Exemplo",
        "open": "Abrir", "save": "Salvar", "clear": "Limpar", "steps": "ETAPAS", "bg_hint": "Os atalhos funcionam mesmo com a janela em segundo plano",
        "cycle": "Um ciclo ≈ {d}", "empty": "Adicione uma etapa à direita ou carregue o exemplo para começar.",
        "running_spam": "Em execução — {n} toques", "running_pat": "Em execução — repetição {i}/{n} · etapa {s}/{t}",
        "no_steps": "Adicione pelo menos uma etapa.", "bad_file": "Arquivo inválido.",
        "pin": "Sempre no topo", "minimize": "Minimizar", "close": "Fechar",
        "language": "Idioma", "left": "Mover para a esquerda", "right": "Mover para a direita",
        "dup": "Duplicar", "del": "Excluir", "clear_q": "Excluir todas as etapas?",
        "Space": "Espaço", "Enter": "Enter", "Esc": "Esc", "Del": "Del",
        "LCtrl": "Ctrl E", "RCtrl": "Ctrl D", "RShift": "Shift D", "Shift": "Shift",
    },
}
_lang = "fr"


def tr(k, **kw):
    s = T.get(_lang, T["fr"]).get(k, T["fr"].get(k, k))
    return s.format(**kw) if kw else s


def fmt_num(v):
    s = f"{v:.3f}".rstrip("0").rstrip(".")
    return s.replace(".", ",") if _lang in ("fr", "pt") else s


# ------------------------------------------------------------------ touches
NAMED = {
    "space": "Space", "enter": "Enter", "esc": "Esc", "delete": "Del", "ctrl_l": "LCtrl",
    "ctrl_r": "RCtrl", "ctrl": "Ctrl", "shift": "Shift", "shift_l": "Shift", "shift_r": "RShift",
    "alt": "Alt", "alt_l": "Alt", "alt_r": "AltGr", "alt_gr": "AltGr", "cmd": "Win",
    "cmd_l": "Win", "cmd_r": "Win", "tab": "Tab", "backspace": "⌫", "caps_lock": "Caps",
    "up": "↑", "down": "↓", "left": "←", "right": "→", "page_up": "PgUp",
    "page_down": "PgDn", "home": "Home", "end": "End", "insert": "Ins",
}
MOD_ORDER = ["ctrl", "shift", "alt", "cmd"]


def key_label(spec):
    if "name" in spec:
        n = spec["name"]
        if n in NAMED:
            v = NAMED[n]
            return tr(v) if v in T["fr"] else v
        return n.upper() if n.startswith("f") and n[1:].isdigit() else n.replace("_", " ").title()
    return spec.get("label", "?")


def spec_from_pynput(k):
    if isinstance(k, Key):
        return {"id": k.name, "name": k.name}
    vk = getattr(k, "vk", None)
    ch = getattr(k, "char", None)
    label = None
    if IS_WIN and vk is not None and (65 <= vk <= 90 or 48 <= vk <= 57):
        label = chr(vk)
    elif IS_WIN and vk is not None and 96 <= vk <= 105:
        label = f"Num {vk - 96}"
    elif ch and ch.isprintable() and not ch.isspace():
        label = ch.upper()
    spec = {"id": f"vk{vk}" if vk is not None else f"ch{(ch or '?').lower()}",
            "label": label or f"VK{vk}"}
    if vk is not None:
        spec["vk"] = vk
    if label and len(label) == 1:
        spec["char"] = label.lower()
    return spec


def spec_letter(c):
    vk = ord(c.upper()) if IS_WIN else ord(c.lower())
    return {"id": f"vk{vk}", "vk": vk, "label": c.upper(), "char": c.lower()}


def spec_named(n):
    return {"id": n, "name": n}


def to_pynput(spec):
    if "name" in spec:
        return Key[spec["name"]]
    if IS_WIN and spec.get("vk") is not None:
        return KeyCode.from_vk(spec["vk"])
    if spec.get("char"):
        return KeyCode.from_char(spec["char"])
    return KeyCode.from_vk(spec["vk"])


def combo_sort(specs):
    def rank(s):
        n = s.get("name", "")
        for i, m in enumerate(MOD_ORDER):
            if n.startswith(m):
                return i
        return len(MOD_ORDER)
    return sorted(specs, key=rank)


def combo_label(specs):
    return "  +  ".join(key_label(s) for s in specs) if specs else tr("none")


# ------------------------------------------------------------------ moteur
class Runner:
    def __init__(self):
        self.thread = None
        self.stop_evt = threading.Event()
        self.held = set()
        self.mode = ""
        self.count = 0
        self.loop = 0
        self.loops = 0
        self.step = -1
        self.steps_total = 0

    @property
    def running(self):
        return self.thread is not None and self.thread.is_alive()

    def start(self, mode, target, *args):
        self.stop_and_wait()
        self.stop_evt.clear()
        self.mode, self.count, self.loop, self.step = mode, 0, 0, -1
        self.thread = threading.Thread(target=self._wrap, args=(target, *args), daemon=True)
        self.thread.start()

    def _wrap(self, target, *args):
        try:
            target(self, *args)
        finally:
            for k in list(self.held):  # ne jamais laisser une touche bloquée
                kb.release(k)
            self.held.clear()
            self.step = -1

    def stop(self):
        self.stop_evt.set()

    def stop_and_wait(self):
        if self.running:
            self.stop()
            self.thread.join(1.0)

    def wait(self, sec):
        return not self.stop_evt.wait(sec)

    def tap(self, k):
        kb.press(k)
        kb.release(k)
        self.count += 1

    def hold(self, k, sec):
        kb.press(k)
        self.held.add(k)
        ok = self.wait(sec)
        kb.release(k)
        self.held.discard(k)
        return ok


def job_spam(r, key, interval):
    while not r.stop_evt.is_set():
        r.tap(key)
        if not r.wait(interval):
            break


def job_pattern(r, steps, loops):
    r.loops, r.steps_total = loops, len(steps)
    while loops == 0 or r.loop < loops:
        r.loop += 1
        for n, s in enumerate(steps):
            r.step = n
            t = s["type"]
            if t == "hold":
                ok = r.hold(s["k"], s["sec"])
            elif t == "press":
                ok = True
                for c in range(s["count"]):
                    r.tap(s["k"])
                    if c < s["count"] - 1 and not r.wait(s["gap"]):
                        ok = False
                        break
            else:
                ok = r.wait(s["sec"])
            if not ok or r.stop_evt.is_set():
                return


# ------------------------------------------------------------------ état / config
def default_state():
    return {
        "lang": "fr", "pin": False, "mode": "spam",
        "spam": {"key": spec_letter("e"), "value": 100, "unit": "ms",
                 "hotkey": [spec_named("f6")], "hk_mode": "toggle"},
        "pattern": {"steps": [], "loops": 0, "hotkey": [spec_named("f7")], "hk_mode": "toggle"},
    }


def example_steps():
    return [
        {"type": "hold", "key": spec_letter("z"), "value": 3, "unit": "s"},
        {"type": "hold", "key": spec_letter("x"), "value": 2, "unit": "s"},
        {"type": "hold", "key": spec_letter("z"), "value": 10, "unit": "s"},
        {"type": "press", "key": spec_letter("e"), "count": 10, "gap": 100},
    ]


def new_step(kind):
    if kind == "hold":
        return {"type": "hold", "key": spec_letter("z"), "value": 1, "unit": "s"}
    if kind == "press":
        return {"type": "press", "key": spec_letter("e"), "count": 5, "gap": 100}
    return {"type": "wait", "value": 1, "unit": "s"}


def config_path():
    base = os.environ.get("APPDATA") or os.path.join(os.path.expanduser("~"), ".config")
    return os.path.join(base, APP_NAME, "config.json")


def load_state():
    st = default_state()
    try:
        with open(config_path(), encoding="utf-8") as fh:
            data = json.load(fh)
        for k in ("lang", "pin", "mode"):
            if k in data:
                st[k] = data[k]
        for k in ("spam", "pattern"):
            st[k].update(data.get(k, {}))
    except Exception:
        pass
    if st["lang"] not in LANGS:
        st["lang"] = "fr"
    return st


def save_state(st):
    try:
        os.makedirs(os.path.dirname(config_path()), exist_ok=True)
        with open(config_path(), "w", encoding="utf-8") as fh:
            json.dump(st, fh, indent=1, ensure_ascii=False)
    except Exception:
        pass


def secs(value, unit):
    return float(value) * UNITS.get(unit, 1)


def fmt_duration(sec):
    if sec < 1:
        return f"{fmt_num(round(sec * 1000))} ms"
    if sec < 60:
        return f"{fmt_num(round(sec, 2))} s"
    m, s = divmod(sec, 60)
    return f"{int(m)} min {fmt_num(round(s, 1))} s" if s >= 0.05 else f"{int(m)} min"


def cycle_seconds(steps):
    total = 0.0
    for s in steps:
        if s["type"] == "press":
            total += max(0, s["count"] - 1) * s["gap"] / 1000
        else:
            total += secs(s["value"], s["unit"])
    return total


# ------------------------------------------------------------------ icônes dessinées
def make_icon(kind, color=None, size=18):
    color = QColor(color or C["text"])
    pm = QPixmap(size * 3, size * 3)
    pm.setDevicePixelRatio(3)
    pm.fill(Qt.transparent)
    p = QPainter(pm)
    p.setRenderHint(QPainter.Antialiasing)
    p.setPen(QPen(color, 1.7, Qt.SolidLine, Qt.RoundCap, Qt.RoundJoin))
    s = size
    P = lambda x, y: QPointF(s * x, s * y)  # noqa: E731
    if kind == "close":
        p.drawLine(P(.3, .3), P(.7, .7)); p.drawLine(P(.7, .3), P(.3, .7))
    elif kind == "min":
        p.drawLine(P(.28, .5), P(.72, .5))
    elif kind == "pin":
        p.drawEllipse(P(.5, .36), s * .15, s * .15); p.drawLine(P(.5, .52), P(.5, .82))
        p.drawLine(P(.3, .52), P(.7, .52))
    elif kind in ("left", "right"):
        path = QPainterPath()
        if kind == "left":
            path.moveTo(P(.6, .28)); path.lineTo(P(.38, .5)); path.lineTo(P(.6, .72))
        else:
            path.moveTo(P(.4, .28)); path.lineTo(P(.62, .5)); path.lineTo(P(.4, .72))
        p.drawPath(path)
    elif kind == "copy":
        p.drawRoundedRect(QRectF(s * .34, s * .34, s * .38, s * .38), 2, 2)
        p.drawPolyline([P(.26, .6), P(.26, .26), P(.6, .26)])
    elif kind == "trash":
        p.drawLine(P(.25, .32), P(.75, .32)); p.drawLine(P(.42, .32), P(.44, .22))
        p.drawLine(P(.44, .22), P(.56, .22)); p.drawLine(P(.56, .22), P(.58, .32))
        p.drawPolyline([P(.32, .32), P(.36, .78), P(.64, .78), P(.68, .32)])
    elif kind == "plus":
        p.drawLine(P(.5, .26), P(.5, .74)); p.drawLine(P(.26, .5), P(.74, .5))
    elif kind == "globe":
        p.drawEllipse(P(.5, .5), s * .3, s * .3); p.drawEllipse(P(.5, .5), s * .13, s * .3)
        p.drawLine(P(.2, .5), P(.8, .5))
    elif kind == "play":
        p.setBrush(color); path = QPainterPath()
        path.moveTo(P(.34, .24)); path.lineTo(P(.78, .5)); path.lineTo(P(.34, .76)); path.closeSubpath()
        p.drawPath(path)
    elif kind == "stop":
        p.setBrush(color); p.drawRoundedRect(QRectF(s * .28, s * .28, s * .44, s * .44), 3, 3)
    elif kind == "hold":
        p.setBrush(color); p.drawRoundedRect(QRectF(s * .2, s * .38, s * .6, s * .24), 4, 4)
    elif kind == "press":
        p.setBrush(color)
        for x in (.27, .5, .73):
            p.drawEllipse(P(x, .5), s * .085, s * .085)
    elif kind == "wait":
        p.drawEllipse(P(.5, .52), s * .28, s * .28); p.drawLine(P(.5, .52), P(.5, .36))
        p.drawLine(P(.5, .52), P(.62, .58)); p.drawLine(P(.42, .16), P(.58, .16))
    elif kind == "open":
        p.drawPolyline([P(.2, .72), P(.2, .3), P(.42, .3), P(.48, .38), P(.8, .38), P(.8, .72), P(.2, .72)])
    elif kind == "save":
        p.drawLine(P(.5, .22), P(.5, .6)); p.drawPolyline([P(.34, .46), P(.5, .62), P(.66, .46)])
        p.drawPolyline([P(.22, .62), P(.22, .78), P(.78, .78), P(.78, .62)])
    elif kind == "spark":
        p.setBrush(color); path = QPainterPath(); path.moveTo(P(.5, .14))
        for a, b in ((.6, .4), (.86, .5), (.6, .6), (.5, .86), (.4, .6), (.14, .5), (.4, .4)):
            path.lineTo(P(a, b))
        path.closeSubpath(); p.drawPath(path)
    elif kind == "loop":
        p.drawArc(QRectF(s * .22, s * .22, s * .56, s * .56), 60 * 16, 300 * 16)
        p.drawPolyline([P(.62, .14), P(.66, .28), P(.52, .32)])
    elif kind == "keyboard":
        p.drawRoundedRect(QRectF(s * .14, s * .3, s * .72, s * .42), 3, 3)
        p.drawLine(P(.32, .6), P(.68, .6))
        for x in (.3, .43, .57, .7):
            p.drawPoint(P(x, .44))
    p.end()
    return QIcon(pm)


def logo_pixmap(n=64):
    pm = QPixmap(n, n)
    pm.fill(Qt.transparent)
    p = QPainter(pm)
    p.setRenderHint(QPainter.Antialiasing)
    g = QLinearGradient(0, 0, n, n)
    g.setColorAt(0, QColor(C["violet"]))
    g.setColorAt(1, QColor(C["cyan"]))
    p.setPen(Qt.NoPen)
    p.setBrush(g)
    p.drawRoundedRect(QRectF(0, 0, n, n), n * .26, n * .26)
    # touche stylisée
    p.setBrush(QColor(255, 255, 255, 235))
    p.drawRoundedRect(QRectF(n * .24, n * .22, n * .52, n * .46), n * .1, n * .1)
    p.setBrush(QColor(255, 255, 255, 120))
    p.drawRoundedRect(QRectF(n * .24, n * .66, n * .52, n * .1), n * .05, n * .05)
    p.setBrush(QColor(C["violet"]))
    path = QPainterPath()  # éclair
    pts = [(.54, .28), (.40, .47), (.50, .47), (.46, .62), (.60, .42), (.50, .42)]
    path.moveTo(n * pts[0][0], n * pts[0][1])
    for x, y in pts[1:]:
        path.lineTo(n * x, n * y)
    path.closeSubpath()
    p.drawPath(path)
    p.end()
    return pm


# ------------------------------------------------------------------ style
QSS = f"""
* {{ font-family: "Segoe UI", "Inter", "DejaVu Sans"; color: {C['text']}; font-size: 13px; }}
#root {{ background: {C['bg']}; border: 1px solid {C['line']}; border-radius: 16px; }}
#appName {{ font-size: 15px; font-weight: 700; }}
#ver {{ color: {C['muted']}; font-size: 11px; }}
#seg {{ background: {C['surface']}; border: 1px solid {C['line']}; border-radius: 10px; }}
#segBtn {{ background: transparent; border: none; border-radius: 7px; padding: 5px 12px;
          color: {C['muted']}; font-weight: 600; }}
#segBtn:hover {{ color: {C['text']}; }}
#segBtn:checked {{ background: {C['violet']}; color: white; }}
#tabs #segBtn {{ padding: 7px 26px; font-size: 13px; }}
#tabs #segBtn:checked {{ background: qlineargradient(x1:0,y1:0,x2:1,y2:1, stop:0 {C['violet']}, stop:1 #6D8BF5); }}
#iconBtn {{ background: transparent; border: none; border-radius: 8px; padding: 6px; }}
#iconBtn:hover {{ background: {C['surface2']}; }}
#iconBtn:checked {{ background: #2A2345; }}
#closeBtn {{ background: transparent; border: none; border-radius: 8px; padding: 6px; }}
#closeBtn:hover {{ background: {C['rose']}; }}
#langBtn {{ background: {C['surface']}; border: 1px solid {C['line']}; border-radius: 9px;
           padding: 5px 10px; font-weight: 600; }}
#langBtn:hover {{ border-color: {C['violet']}; }}
#langBtn::menu-indicator {{ image: none; width: 0; }}
#card {{ background: {C['surface']}; border: 1px solid #1F2431; border-radius: 16px; }}
#cardTitle {{ color: {C['muted']}; font-size: 11px; font-weight: 700; }}
#capBig {{ background: qlineargradient(x1:0,y1:0,x2:0,y2:1, stop:0 #1D2230, stop:1 #161A23);
          border: 1px solid {C['line2']}; border-bottom: 6px solid #0E1118; border-radius: 16px;
          font-size: 44px; font-weight: 800; padding: 8px; }}
#capBig:hover, #cap:hover {{ border-color: {C['violet']}; }}
#capBig[recording="true"] {{ border: 2px solid {C['cyan']}; color: {C['cyan']}; font-size: 13px; }}
#cap {{ background: {C['surface2']}; border: 1px solid {C['line2']}; border-radius: 11px;
       font-size: 15px; font-weight: 700; padding: 8px 12px; }}
#cap[recording="true"] {{ border: 2px solid {C['cyan']}; color: {C['cyan']}; font-size: 12px; }}
#capStep {{ background: qlineargradient(x1:0,y1:0,x2:0,y2:1, stop:0 #1D2230, stop:1 #161A23);
           border: 1px solid {C['line2']}; border-bottom: 4px solid #0E1118; border-radius: 11px;
           font-size: 22px; font-weight: 800; padding: 4px; }}
#capStep:hover {{ border-color: {C['violet']}; }}
#capStep[recording="true"] {{ border: 2px solid {C['cyan']}; color: {C['cyan']}; font-size: 11px; }}
QLineEdit {{ background: {C['surface2']}; border: 1px solid {C['line2']}; border-radius: 10px;
            padding: 4px 6px; font-weight: 700; selection-background-color: {C['violet']}; }}
QLineEdit:focus {{ border-color: {C['violet']}; }}
QLineEdit:disabled {{ color: #4A5066; }}
#bigSpin {{ font-size: 32px; }}
#midSpin {{ font-size: 17px; }}
#chip {{ background: transparent; border: 1px solid {C['line2']}; border-radius: 12px;
        padding: 3px 10px; color: #A9AEC2; font-size: 12px; font-weight: 600; }}
#chip:hover {{ border-color: {C['violet']}; color: {C['text']}; }}
#muted {{ color: {C['muted']}; font-size: 12px; }}
#accentInfo {{ color: {C['cyan']}; font-size: 12px; font-weight: 600; }}
#start {{ border: none; border-radius: 18px;
         background: qlineargradient(x1:0,y1:0,x2:1,y2:1, stop:0 {C['violet']}, stop:1 {C['cyan']}); }}
#start:hover {{ background: qlineargradient(x1:0,y1:0,x2:1,y2:1, stop:0 #9D74FF, stop:1 #4FE0F5); }}
#start[running="true"] {{ background: qlineargradient(x1:0,y1:0,x2:1,y2:1, stop:0 {C['rose']}, stop:1 {C['orange']}); }}
#startText {{ color: white; font-size: 18px; font-weight: 700; background: transparent; }}
#startHint {{ color: rgba(255,255,255,0.8); font-size: 12px; font-weight: 600; background: transparent; }}
#footer {{ border-top: 1px solid #1A1F2A; }}
#status {{ font-size: 12px; font-weight: 600; }}
#step {{ background: {C['surface']}; border: 1px solid #1F2431; border-radius: 16px; }}
#step[active="true"] {{ border: 2px solid {C['cyan']}; background: #101B22; }}
#stepTitle {{ font-weight: 700; font-size: 13px; }}
#stepNum {{ color: {C['muted']}; font-weight: 700; font-size: 12px; }}
#mini {{ background: transparent; border: none; border-radius: 6px; padding: 2px; }}
#mini:hover {{ background: {C['line']}; }}
#miniDanger {{ background: transparent; border: none; border-radius: 6px; padding: 2px; }}
#miniDanger:hover {{ background: #3A1620; }}
#addCard {{ background: transparent; border: 2px dashed #262C3B; border-radius: 16px; }}
#addBtn {{ background: {C['surface2']}; border: 1px solid {C['line2']}; border-radius: 11px;
          padding: 9px 12px; text-align: left; font-weight: 700; }}
#addBtn:hover {{ border-color: {C['violet']}; background: #1C2030; }}
#toolBtn {{ background: {C['surface2']}; border: 1px solid {C['line2']}; border-radius: 10px;
           padding: 7px 11px; font-weight: 600; }}
#toolBtn:hover {{ border-color: {C['violet']}; }}
#toolBtn:checked {{ background: #2A2345; border-color: {C['violet']}; }}
#emptyHint {{ color: {C['muted']}; font-size: 14px; }}
QScrollArea, #timelineInner, #vp {{ background: transparent; border: none; }}
QScrollBar:horizontal {{ height: 8px; background: transparent; margin: 0 6px; }}
QScrollBar::handle:horizontal {{ background: #2A3042; border-radius: 4px; min-width: 40px; }}
QScrollBar::handle:horizontal:hover {{ background: {C['violet']}; }}
QScrollBar::add-line, QScrollBar::sub-line {{ width: 0; height: 0; }}
QScrollBar::add-page, QScrollBar::sub-page {{ background: transparent; }}
QToolTip {{ background: {C['surface2']}; color: {C['text']}; border: 1px solid {C['line2']}; padding: 6px; }}
QMenu {{ background: {C['surface']}; border: 1px solid {C['line2']}; padding: 6px; }}
QMenu::item {{ padding: 7px 22px; border-radius: 6px; }}
QMenu::item:selected {{ background: {C['violet']}; }}
QMessageBox {{ background: {C['surface']}; }}
QMessageBox QPushButton {{ background: {C['surface2']}; border: 1px solid {C['line2']}; border-radius: 8px;
                          padding: 6px 16px; min-width: 60px; }}
"""


def repolish(w):
    w.style().unpolish(w)
    w.style().polish(w)
    w.update()


def card_title(text):
    lab = QLabel(text)
    lab.setObjectName("cardTitle")
    f = lab.font()
    f.setLetterSpacing(QFont.AbsoluteSpacing, 1.2)
    lab.setFont(f)
    return lab


def label(text, obj=None, wrap=False):
    lab = QLabel(text)
    if obj:
        lab.setObjectName(obj)
    lab.setWordWrap(wrap)
    return lab


# ------------------------------------------------------------------ widgets
class Segmented(QFrame):
    changed = Signal(str)

    def __init__(self, items, current, expand=False):
        super().__init__()
        self.setObjectName("seg")
        lay = QHBoxLayout(self)
        lay.setContentsMargins(3, 3, 3, 3)
        lay.setSpacing(2)
        self.group = QButtonGroup(self)
        self.btns = {}
        for value, text in items:
            b = QPushButton(text)
            b.setObjectName("segBtn")
            b.setSizePolicy(QSizePolicy.Expanding if expand else QSizePolicy.Fixed, QSizePolicy.Fixed)
            b.setCheckable(True)
            b.setCursor(Qt.PointingHandCursor)
            b.setChecked(value == current)
            b.clicked.connect(lambda _=False, v=value: self.changed.emit(v))
            self.group.addButton(b)
            self.btns[value] = b
            lay.addWidget(b)

    def set_value(self, v):
        if v in self.btns:
            self.btns[v].setChecked(True)


class NumEdit(QLineEdit):
    """Champ numérique stylable : saisie libre (virgule ou point), molette = +/- 1."""
    valueChanged = Signal(float)

    def __init__(self, value, lo=0.0, hi=999999.0, integer=False, obj="midSpin"):
        super().__init__()
        self.setObjectName(obj)
        self.lo, self.hi, self.integer = lo, hi, integer
        self._v = None
        self.setAlignment(Qt.AlignCenter)
        self.setValue(value)
        self.editingFinished.connect(self._commit)
        self.textEdited.connect(self._live)

    def value(self):
        return self._v

    def _parse(self, txt):
        try:
            v = float(txt.strip().replace(",", "."))
        except ValueError:
            return None
        v = min(self.hi, max(self.lo, v))
        return float(int(round(v))) if self.integer else v

    def _live(self, txt):
        v = self._parse(txt)
        if v is not None and v != self._v:
            self._v = v
            self.valueChanged.emit(v)

    def _commit(self):
        v = self._parse(self.text())
        self.setValue(self._v if v is None else v)

    def setValue(self, v):
        v = self._parse(str(v))
        changed = v != self._v
        self._v = v
        self.setText(str(int(v)) if self.integer else fmt_num(v))
        if changed:
            self.valueChanged.emit(v)

    def setEnabled(self, on):
        super().setEnabled(on)

    def wheelEvent(self, ev):
        step = 1 if ev.angleDelta().y() > 0 else -1
        self.setValue(self._v + step)
        ev.accept()


def IntSpin(value, lo=0, hi=999999, obj="midSpin"):
    return NumEdit(value, lo, hi, integer=True, obj=obj)


def NumSpin(value, lo=0, hi=999999, decimals=3, obj="midSpin"):
    return NumEdit(value, lo, hi, obj=obj)


class KeyCapture(QPushButton):
    """Bouton : clic → écoute du clavier → enregistre une touche (ou un combo)."""
    captured = Signal(list)

    def __init__(self, hub, specs, combo=False, obj="cap"):
        super().__init__()
        self.hub, self.combo, self.specs = hub, combo, list(specs)
        self.setObjectName(obj)
        self.setCursor(Qt.PointingHandCursor)
        self.setProperty("recording", False)
        self.setToolTip(tr("click_change"))
        self.clicked.connect(lambda: self.hub.begin_record(self))
        self.refresh()

    def refresh(self, preview=None):
        if preview is not None:
            self.setText(combo_label(preview) if preview else (tr("press_combo") if self.combo else tr("press_key")))
        else:
            self.setText(combo_label(self.specs))

    def set_recording(self, on):
        self.setProperty("recording", on)
        repolish(self)
        self.refresh([] if on else None)

    def finish(self, specs):
        self.specs = specs
        self.set_recording(False)
        self.captured.emit(specs)


class StartButton(QPushButton):
    def __init__(self, hint, w=170):
        super().__init__()
        self.setObjectName("start")
        self.setCursor(Qt.PointingHandCursor)
        self.setProperty("running", False)
        self.setMinimumWidth(w)
        self.setSizePolicy(QSizePolicy.Fixed, QSizePolicy.Expanding)
        lay = QVBoxLayout(self)
        lay.setContentsMargins(10, 10, 10, 10)
        lay.setSpacing(6)
        lay.addStretch()
        self.ico = QLabel()
        self.ico.setAlignment(Qt.AlignCenter)
        self.txt = label("", "startText")
        self.txt.setAlignment(Qt.AlignCenter)
        self.hint = label(hint, "startHint")
        self.hint.setAlignment(Qt.AlignCenter)
        for w_ in (self.ico, self.txt, self.hint):
            w_.setAttribute(Qt.WA_TransparentForMouseEvents)
            w_.setStyleSheet("background: transparent;")
            lay.addWidget(w_)
        lay.addStretch()
        self._running = None
        self.set_running(False)

    def set_running(self, on):
        if on == self._running:
            return
        self._running = on
        self.setProperty("running", on)
        repolish(self)
        self.ico.setPixmap(make_icon("stop" if on else "play", "white", 30).pixmap(30, 30))
        self.txt.setText(tr("stop") if on else tr("start"))


class StatusDot(QWidget):
    def __init__(self):
        super().__init__()
        self.setFixedSize(14, 14)
        self.running = False
        self.phase = 0.0

    def tick(self, running):
        self.running = running
        self.phase = (self.phase + 0.08) % 2
        self.update()

    def paintEvent(self, _):
        p = QPainter(self)
        p.setRenderHint(QPainter.Antialiasing)
        col = QColor(C["green"] if self.running else C["muted"])
        if self.running:
            a = abs(1 - self.phase)
            halo = QColor(col)
            halo.setAlphaF(0.15 + 0.3 * a)
            p.setPen(Qt.NoPen)
            p.setBrush(halo)
            p.drawEllipse(QPointF(7, 7), 7, 7)
        p.setPen(Qt.NoPen)
        p.setBrush(col)
        p.drawEllipse(QPointF(7, 7), 3.5, 3.5)


class RhythmBar(QWidget):
    """Visualise le rythme : une barre par pression sur une fenêtre de temps."""
    def __init__(self):
        super().__init__()
        self.setFixedHeight(46)
        self.iv = 0.1

    def set_interval(self, iv):
        self.iv = max(0.001, iv)
        self.update()

    def paintEvent(self, _):
        p = QPainter(self)
        p.setRenderHint(QPainter.Antialiasing)
        w, h = self.width(), self.height()
        win = 1.0 if self.iv <= 1 else (60.0 if self.iv <= 60 else 3600.0)
        unit = "1 s" if win == 1 else ("1 min" if win == 60 else "1 h")
        track = QRectF(0, 6, w, 20)
        p.setPen(Qt.NoPen)
        p.setBrush(QColor(C["surface2"]))
        p.drawRoundedRect(track, 8, 8)
        n = int(win / self.iv)
        g = QLinearGradient(0, 0, w, 0)
        g.setColorAt(0, QColor(C["violet"]))
        g.setColorAt(1, QColor(C["cyan"]))
        if n > 80:  # trop dense : barre pleine hachurée
            p.setBrush(g)
            p.setOpacity(0.85)
            p.drawRoundedRect(track.adjusted(3, 3, -3, -3), 6, 6)
            p.setOpacity(1)
        else:
            p.setBrush(g)
            span = w - 16
            for i in range(n + 1):
                x = 8 + span * (i * self.iv / win)
                if x > w - 6:
                    break
                p.drawRoundedRect(QRectF(x - 1.5, 10, 3, 12), 1.5, 1.5)
        p.setPen(QColor(C["muted"]))
        f = p.font()
        f.setPixelSize(11)
        p.setFont(f)
        p.drawText(QRectF(2, 28, 60, 16), Qt.AlignLeft | Qt.AlignVCenter, "0")
        p.drawText(QRectF(w - 62, 28, 60, 16), Qt.AlignRight | Qt.AlignVCenter, unit)


def make_card(title=None, margins=(18, 16, 18, 16), spacing=10):
    f = QFrame()
    f.setObjectName("card")
    lay = QVBoxLayout(f)
    lay.setContentsMargins(*margins)
    lay.setSpacing(spacing)
    if title:
        lay.addWidget(card_title(title))
    return f, lay


def icon_button(kind, tip, obj="iconBtn", color=None, size=18):
    b = QPushButton()
    b.setObjectName(obj)
    b.setIcon(make_icon(kind, color, size))
    b.setIconSize(QSize(size, size))
    b.setToolTip(tip)
    b.setCursor(Qt.PointingHandCursor)
    return b


# ------------------------------------------------------------------ carte d'étape
class StepCard(QFrame):
    action = Signal(str, int)  # left/right/dup/del, index
    edited = Signal()

    def __init__(self, hub, step, index, total):
        super().__init__()
        self.step, self.index = step, index
        self.setObjectName("step")
        self.setProperty("active", False)
        self.setFixedSize(214, 238)
        kind = step["type"]
        col = KIND_COLOR[kind]

        lay = QVBoxLayout(self)
        lay.setContentsMargins(14, 12, 12, 12)
        lay.setSpacing(9)

        head = QHBoxLayout()
        head.setSpacing(6)
        badge = QLabel()
        badge.setFixedSize(26, 26)
        badge.setAlignment(Qt.AlignCenter)
        qc = QColor(col)
        badge.setStyleSheet(f"background: rgba({qc.red()},{qc.green()},{qc.blue()},40); border-radius: 8px;")
        badge.setPixmap(make_icon(kind, col, 18).pixmap(18, 18))
        head.addWidget(badge)
        num = label(str(index + 1), "stepNum")
        title = label(tr("k_" + kind), "stepTitle")
        title.setStyleSheet(f"color: {col};")
        head.addWidget(title)
        head.addWidget(num)
        head.addStretch()
        delb = icon_button("trash", tr("del"), "miniDanger", "#F87185", 16)
        delb.setFixedSize(26, 26)
        delb.clicked.connect(lambda: self.action.emit("trash", self.index))
        head.addWidget(delb)
        lay.addLayout(head)

        if kind != "wait":
            self.cap = KeyCapture(hub, [step["key"]], combo=False, obj="capStep")
            self.cap.setFixedHeight(52)
            self.cap.captured.connect(self._on_key)
            lay.addWidget(self.cap)
        else:
            ico = QLabel()
            ico.setAlignment(Qt.AlignCenter)
            ico.setFixedHeight(52)
            ico.setPixmap(make_icon("wait", col, 40).pixmap(40, 40))
            lay.addWidget(ico)

        if kind in ("hold", "wait"):
            row = QHBoxLayout()
            row.setSpacing(6)
            sp = NumSpin(step["value"])
            sp.setFixedWidth(96)
            sp.valueChanged.connect(lambda v: self._set("value", v))
            seg = Segmented([(u, u) for u in UNITS], step["unit"], expand=True)
            seg.changed.connect(lambda u: self._set("unit", u))
            row.addWidget(label(tr("for"), "muted"))
            row.addStretch()
            row.addWidget(sp)
            lay.addLayout(row)
            lay.addWidget(seg)
        else:
            row = QHBoxLayout()
            row.setSpacing(6)
            cnt = IntSpin(step["count"], 1, 100000)
            cnt.setFixedWidth(96)
            cnt.valueChanged.connect(lambda v: self._set("count", int(v)))
            row.addStretch()
            row.addWidget(cnt)
            row.addWidget(label(tr("times"), "muted"))
            lay.addLayout(row)
            row2 = QHBoxLayout()
            row2.setSpacing(6)
            gap = NumSpin(step["gap"], 0, 600000)
            gap.setFixedWidth(96)
            gap.valueChanged.connect(lambda v: self._set("gap", v))
            row2.addWidget(label(tr("gap"), "muted"))
            row2.addStretch()
            row2.addWidget(gap)
            row2.addWidget(label("ms", "muted"))
            lay.addLayout(row2)
        lay.addStretch()
        foot = QHBoxLayout()
        foot.setSpacing(2)
        for k, tip, enabled in (("left", tr("left"), index > 0), ("right", tr("right"), index < total - 1)):
            b = icon_button(k, tip, "mini", C["muted"] if enabled else "#343A4C", 16)
            b.setFixedSize(26, 24)
            b.setEnabled(enabled)
            b.clicked.connect(lambda _=False, a=k: self.action.emit(a, self.index))
            foot.addWidget(b)
        foot.addStretch()
        cp = icon_button("copy", tr("dup"), "mini", C["muted"], 16)
        cp.setFixedSize(26, 24)
        cp.clicked.connect(lambda: self.action.emit("copy", self.index))
        foot.addWidget(cp)
        lay.addLayout(foot)

    def _on_key(self, specs):
        self.step["key"] = specs[0]
        self.edited.emit()

    def _set(self, k, v):
        self.step[k] = v
        self.edited.emit()

    def set_active(self, on):
        if self.property("active") != on:
            self.setProperty("active", on)
            repolish(self)


# ------------------------------------------------------------------ pont clavier global
class KeyBridge(QObject):
    down = Signal(dict)
    up = Signal(dict)


# ------------------------------------------------------------------ fenêtre principale
class MainWindow(QWidget):
    def __init__(self):
        super().__init__()
        global _lang
        self.state = load_state()
        _lang = self.state["lang"]
        self.runner = Runner()
        self.recorder = None
        self.rec_keys, self.rec_down = [], set()
        self.pressed = set()
        self.bind_active = {"spam": False, "pattern": False}
        self.step_cards = []

        self.setWindowTitle(APP_NAME)
        self.setWindowIcon(QIcon(logo_pixmap(256)))
        self.setWindowFlags(Qt.FramelessWindowHint | Qt.Window)
        self.setAttribute(Qt.WA_TranslucentBackground)
        self.apply_size()
        if self.state["pin"]:
            self.setWindowFlag(Qt.WindowStaysOnTopHint, True)

        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        self.root = QFrame()
        self.root.setObjectName("root")
        outer.addWidget(self.root)
        self.root_lay = QVBoxLayout(self.root)
        self.root_lay.setContentsMargins(0, 0, 0, 0)
        self.root_lay.setSpacing(0)
        self.body = None
        self.save_timer = QTimer(self, singleShot=True, interval=400, timeout=lambda: save_state(self.state))
        self.build()

        self.poll_timer = QTimer(self, interval=50, timeout=self.poll)
        self.poll_timer.start()

        # écoute clavier globale (raccourcis + capture)
        self.bridge = KeyBridge()
        self.bridge.down.connect(self.on_key_down)
        self.bridge.up.connect(self.on_key_up)
        self.listener = keyboard.Listener(on_press=self._lp, on_release=self._lr)
        self.listener.daemon = True
        self.listener.start()

    # ---- thread pynput → Qt
    def _lp(self, key, injected=False):
        if not injected and key is not None:
            self.bridge.down.emit(spec_from_pynput(key))

    def _lr(self, key, injected=False):
        if not injected and key is not None:
            self.bridge.up.emit(spec_from_pynput(key))

    def dirty(self):
        self.save_timer.start()

    # ---- construction
    def build(self):
        if self.body is not None:
            self.body.setParent(None)
            self.body.deleteLater()
        self.body = QWidget()
        lay = QVBoxLayout(self.body)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.setSpacing(0)
        lay.addWidget(self.build_titlebar())
        self.stack = QStackedWidget()
        self.stack.addWidget(self.build_spam())
        self.stack.addWidget(self.build_pattern())
        self.stack.setCurrentIndex(0 if self.state["mode"] == "spam" else 1)
        lay.addWidget(self.stack, 1)
        lay.addWidget(self.build_footer())
        self.root_lay.addWidget(self.body)

    def build_titlebar(self):
        bar = QFrame()
        bar.setObjectName("titlebar")
        bar.setFixedHeight(60)
        bar.mousePressEvent = self._drag
        lay = QHBoxLayout(bar)
        lay.setContentsMargins(18, 0, 12, 0)
        lay.setSpacing(8)

        left = QHBoxLayout()
        left.setSpacing(10)
        logo = QLabel()
        logo.setPixmap(logo_pixmap(56).scaled(28, 28, Qt.KeepAspectRatio, Qt.SmoothTransformation))
        left.addWidget(logo)
        left.addWidget(label(APP_NAME, "appName"))
        left.addWidget(label("v" + VERSION, "ver"))
        left.addStretch()
        lw = QWidget()
        lw.setLayout(left)
        lw.setFixedWidth(260)
        lay.addWidget(lw)
        lay.addStretch()

        self.tabs = Segmented([("spam", tr("spam")), ("pattern", tr("pattern"))], self.state["mode"])
        self.tabs.setObjectName("tabs")
        self.tabs.changed.connect(self.set_mode)
        lay.addWidget(self.tabs)
        lay.addStretch()

        right = QHBoxLayout()
        right.setSpacing(4)
        right.addStretch()
        lang = QPushButton(" " + self.state["lang"].upper())
        lang.setObjectName("langBtn")
        lang.setIcon(make_icon("globe", C["text"], 16))
        lang.setToolTip(tr("language"))
        lang.setCursor(Qt.PointingHandCursor)
        menu = QMenu(lang)
        for code, name in LANGS.items():
            menu.addAction(name, lambda c=code: self.set_lang(c))
        lang.setMenu(menu)
        right.addWidget(lang)
        right.addSpacing(6)
        pin = icon_button("pin", tr("pin"))
        pin.setCheckable(True)
        pin.setChecked(self.state["pin"])
        pin.toggled.connect(self.set_pin)
        right.addWidget(pin)
        mn = icon_button("min", tr("minimize"))
        mn.clicked.connect(self.showMinimized)
        right.addWidget(mn)
        cl = icon_button("close", tr("close"), "closeBtn")
        cl.clicked.connect(self.close)
        right.addWidget(cl)
        rw = QWidget()
        rw.setLayout(right)
        rw.setFixedWidth(260)
        lay.addWidget(rw)
        return bar

    def build_footer(self):
        f = QFrame()
        f.setObjectName("footer")
        f.setFixedHeight(40)
        lay = QHBoxLayout(f)
        lay.setContentsMargins(20, 0, 20, 0)
        lay.setSpacing(8)
        self.dot = StatusDot()
        lay.addWidget(self.dot)
        self.status = label(tr("idle"), "status")
        lay.addWidget(self.status)
        lay.addStretch()
        self.footer_info = label(tr("bg_hint"), "muted")
        lay.addWidget(self.footer_info)
        return f

    # ---- vue spam
    def build_spam(self):
        st = self.state["spam"]
        page = QWidget()
        lay = QHBoxLayout(page)
        lay.setContentsMargins(22, 10, 22, 20)
        lay.setSpacing(14)

        # touche
        c1, l1 = make_card(tr("key"))
        c1.setFixedWidth(180)
        cap = KeyCapture(self, [st["key"]], obj="capBig")
        cap.setSizePolicy(QSizePolicy.Expanding, QSizePolicy.Expanding)
        cap.captured.connect(lambda s: (st.__setitem__("key", s[0]), self.dirty()))
        l1.addWidget(cap, 1)
        hint = label(tr("click_change"), "muted")
        hint.setAlignment(Qt.AlignCenter)
        l1.addWidget(hint)
        lay.addWidget(c1)

        # intervalle
        c2, l2 = make_card(tr("interval"))
        l2.addWidget(label(tr("every"), "muted"))
        row = QHBoxLayout()
        row.setSpacing(10)
        self.spam_spin = NumSpin(st["value"], 0, 999999, obj="bigSpin")
        self.spam_spin.setFixedHeight(64)
        self.spam_spin.valueChanged.connect(lambda v: self._spam_set("value", v))
        row.addWidget(self.spam_spin, 1)
        self.spam_unit = Segmented([(u, u) for u in UNITS], st["unit"])
        self.spam_unit.changed.connect(lambda u: self._spam_set("unit", u))
        row.addWidget(self.spam_unit)
        l2.addLayout(row)
        chips = QHBoxLayout()
        chips.setSpacing(6)
        for v, u in ((50, "ms"), (100, "ms"), (500, "ms"), (1, "s"), (1, "min")):
            b = QPushButton(f"{v} {u}")
            b.setObjectName("chip")
            b.setSizePolicy(QSizePolicy.Fixed, QSizePolicy.Fixed)
            b.setCursor(Qt.PointingHandCursor)
            b.clicked.connect(lambda _=False, v=v, u=u: self._spam_preset(v, u))
            chips.addWidget(b)
        chips.addStretch()
        l2.addLayout(chips)
        l2.addStretch()
        self.rhythm = RhythmBar()
        l2.addWidget(self.rhythm)
        self.rate = label("", "accentInfo")
        l2.addWidget(self.rate)
        lay.addWidget(c2, 1)

        # raccourci
        c3, l3 = self.build_hotkey_card("spam")
        c3.setFixedWidth(250)
        lay.addWidget(c3)

        self.start_spam = StartButton(combo_label(st["hotkey"]), 160)
        self.start_spam.clicked.connect(lambda: self.toggle("spam"))
        lay.addWidget(self.start_spam)
        self.update_rate()
        return page

    def build_hotkey_card(self, which):
        st = self.state[which]
        c, l = make_card(tr("hotkey"))
        cap = KeyCapture(self, st["hotkey"], combo=True, obj="cap")
        cap.setMinimumHeight(64)

        def on_cap(specs, which=which):
            st["hotkey"] = specs
            self.bind_active[which] = False
            (self.start_spam if which == "spam" else self.start_pat).hint.setText(combo_label(specs))
            self.dirty()
        cap.captured.connect(on_cap)
        l.addWidget(cap)
        seg = Segmented([("toggle", tr("toggle")), ("hold", tr("hold"))], st["hk_mode"], expand=True)
        hint = label(tr("hk_" + st["hk_mode"] + "_hint"), "muted", wrap=True)

        def on_mode(m, which=which, hint=hint):
            st["hk_mode"] = m
            hint.setText(tr("hk_" + m + "_hint"))
            self.dirty()
        seg.changed.connect(on_mode)
        l.addWidget(seg)
        l.addWidget(hint)
        l.addStretch()
        return c, l

    def _spam_set(self, k, v):
        self.state["spam"][k] = v
        self.update_rate()
        self.dirty()

    def _spam_preset(self, v, u):
        self.state["spam"]["unit"] = u
        self.spam_unit.set_value(u)
        self.spam_spin.setValue(v)
        self._spam_set("value", v)

    def update_rate(self):
        iv = max(0.001, secs(self.state["spam"]["value"], self.state["spam"]["unit"]))
        self.rhythm.set_interval(iv)
        if iv <= 1:
            self.rate.setText(tr("rate_s", n=fmt_num(round(1 / iv, 1))))
        elif iv <= 60:
            self.rate.setText(tr("rate_m", n=fmt_num(round(60 / iv, 1))))
        else:
            self.rate.setText(tr("rate_h", n=fmt_num(round(3600 / iv, 1))))

    # ---- vue pattern
    def build_pattern(self):
        st = self.state["pattern"]
        page = QWidget()
        lay = QVBoxLayout(page)
        lay.setContentsMargins(22, 4, 22, 16)
        lay.setSpacing(10)

        header = QHBoxLayout()
        header.setSpacing(6)
        header.addWidget(card_title(tr("steps")))
        header.addSpacing(10)
        self.cycle_lbl = label("", "accentInfo")
        header.addWidget(self.cycle_lbl)
        header.addStretch()
        for kind, txt, fn in (("spark", tr("example"), self.load_example), ("open", tr("open"), self.load_preset),
                              ("save", tr("save"), self.save_preset), ("trash", tr("clear"), self.clear_steps)):
            b = QPushButton(" " + txt)
            b.setObjectName("toolBtn")
            b.setIcon(make_icon(kind, C["amber"] if kind == "spark" else C["text"], 16))
            b.setCursor(Qt.PointingHandCursor)
            b.clicked.connect(fn)
            header.addWidget(b)
        lay.addLayout(header)

        self.scroll = QScrollArea()
        self.scroll.setWidgetResizable(True)
        self.scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarAsNeeded)
        self.scroll.setVerticalScrollBarPolicy(Qt.ScrollBarAlwaysOff)
        self.scroll.setFixedHeight(258)
        self.scroll.viewport().setObjectName("vp")
        lay.addWidget(self.scroll)
        self.refresh_timeline()

        bottom = QHBoxLayout()
        bottom.setSpacing(12)

        # boucles
        c1, l1 = make_card(tr("loops"), (16, 12, 16, 12), 8)
        row = QHBoxLayout()
        row.setSpacing(6)
        minus = icon_button("min", "", "toolBtn", size=16)
        self.loop_spin = IntSpin(max(1, st["loops"]) if st["loops"] else 1, 1, 999999)
        self.loop_spin.setFixedWidth(80)
        plus = icon_button("plus", "", "toolBtn", size=16)
        inf = QPushButton(" " + tr("infinite"))
        inf.setObjectName("toolBtn")
        inf.setIcon(make_icon("loop", C["text"], 16))
        inf.setCheckable(True)
        inf.setChecked(st["loops"] == 0)
        inf.setCursor(Qt.PointingHandCursor)
        self.loop_spin.setEnabled(st["loops"] != 0)
        minus.clicked.connect(lambda: self.loop_spin.setValue(self.loop_spin.value() - 1))
        plus.clicked.connect(lambda: self.loop_spin.setValue(self.loop_spin.value() + 1))

        def loops_changed(*_):
            st["loops"] = 0 if inf.isChecked() else int(self.loop_spin.value())
            self.loop_spin.setEnabled(not inf.isChecked())
            minus.setEnabled(not inf.isChecked())
            plus.setEnabled(not inf.isChecked())
            self.dirty()
        self.loop_spin.valueChanged.connect(loops_changed)
        inf.toggled.connect(loops_changed)
        loops_changed()
        for w in (minus, self.loop_spin, plus, inf):
            row.addWidget(w)
        l1.addLayout(row)
        bottom.addWidget(c1)

        # raccourci (version horizontale compacte)
        c2, l2 = make_card(tr("hotkey"), (16, 12, 16, 12), 8)
        row2 = QHBoxLayout()
        row2.setSpacing(8)
        cap = KeyCapture(self, st["hotkey"], combo=True, obj="cap")
        cap.setMinimumWidth(150)

        def on_cap(specs):
            st["hotkey"] = specs
            self.bind_active["pattern"] = False
            self.start_pat.hint.setText(combo_label(specs))
            self.dirty()
        cap.captured.connect(on_cap)
        seg = Segmented([("toggle", tr("toggle")), ("hold", tr("hold"))], st["hk_mode"])
        seg.changed.connect(lambda m: (st.__setitem__("hk_mode", m), self.dirty()))
        seg.setToolTip(tr("hk_toggle_hint") + "\n" + tr("hk_hold_hint"))
        row2.addWidget(cap, 1)
        row2.addWidget(seg)
        l2.addLayout(row2)
        bottom.addWidget(c2, 1)

        self.start_pat = StartButton(combo_label(st["hotkey"]), 190)
        self.start_pat.setFixedHeight(92)
        self.start_pat.layout().setSpacing(2)
        self.start_pat.ico.hide()
        self.start_pat.clicked.connect(lambda: self.toggle("pattern"))
        bottom.addWidget(self.start_pat)
        lay.addLayout(bottom)
        return page

    def refresh_timeline(self, scroll_end=False):
        steps = self.state["pattern"]["steps"]
        inner = QWidget()
        inner.setObjectName("timelineInner")
        row = QHBoxLayout(inner)
        row.setContentsMargins(0, 2, 0, 8)
        row.setSpacing(12)
        self.step_cards = []
        if not steps:
            empty = QFrame()
            empty.setObjectName("card")
            el = QVBoxLayout(empty)
            el.setContentsMargins(30, 20, 30, 20)
            el.addStretch()
            ico = QLabel()
            ico.setPixmap(make_icon("keyboard", C["muted"], 44).pixmap(44, 44))
            ico.setAlignment(Qt.AlignCenter)
            el.addWidget(ico)
            h = label(tr("empty"), "emptyHint", wrap=True)
            h.setAlignment(Qt.AlignCenter)
            el.addWidget(h)
            ex = QPushButton(" " + tr("example"))
            ex.setObjectName("toolBtn")
            ex.setIcon(make_icon("spark", C["amber"], 16))
            ex.setCursor(Qt.PointingHandCursor)
            ex.clicked.connect(self.load_example)
            el.addWidget(ex, 0, Qt.AlignCenter)
            el.addStretch()
            empty.setFixedHeight(238)
            row.addWidget(empty, 1)
        for i, s in enumerate(steps):
            card = StepCard(self, s, i, len(steps))
            card.action.connect(self.step_action)
            card.edited.connect(self.on_steps_edited)
            self.step_cards.append(card)
            row.addWidget(card)
            if i < len(steps) - 1:
                arrow = QLabel()
                arrow.setPixmap(make_icon("right", "#3A4054", 16).pixmap(16, 16))
                row.addWidget(arrow)
                row.setSpacing(6)

        add = QFrame()
        add.setObjectName("addCard")
        add.setFixedSize(190, 238)
        al = QVBoxLayout(add)
        al.setContentsMargins(14, 14, 14, 14)
        al.setSpacing(8)
        t = label(tr("add"), "stepTitle")
        t.setAlignment(Qt.AlignCenter)
        al.addWidget(t)
        al.addStretch()
        for kind in ("hold", "press", "wait"):
            b = QPushButton("  " + tr("k_" + kind))
            b.setObjectName("addBtn")
            b.setIcon(make_icon(kind, KIND_COLOR[kind], 18))
            b.setCursor(Qt.PointingHandCursor)
            b.clicked.connect(lambda _=False, k=kind: self.add_step(k))
            al.addWidget(b)
        al.addStretch()
        row.addWidget(add)
        if steps:
            row.addStretch()
        self.scroll.setWidget(inner)
        if scroll_end:
            QTimer.singleShot(30, lambda: self.scroll.horizontalScrollBar().setValue(
                self.scroll.horizontalScrollBar().maximum()))
        self.update_cycle()

    def update_cycle(self):
        steps = self.state["pattern"]["steps"]
        self.cycle_lbl.setText(tr("cycle", d=fmt_duration(cycle_seconds(steps))) if steps else "")

    def on_steps_edited(self):
        self.update_cycle()
        self.dirty()

    def add_step(self, kind):
        self.state["pattern"]["steps"].append(new_step(kind))
        self.refresh_timeline(scroll_end=True)
        self.dirty()

    def step_action(self, act, i):
        steps = self.state["pattern"]["steps"]
        if act == "left" and i > 0:
            steps[i - 1], steps[i] = steps[i], steps[i - 1]
        elif act == "right" and i < len(steps) - 1:
            steps[i + 1], steps[i] = steps[i], steps[i + 1]
        elif act == "copy":
            steps.insert(i + 1, json.loads(json.dumps(steps[i])))
        elif act == "trash":
            del steps[i]
        pos = self.scroll.horizontalScrollBar().value()
        self.refresh_timeline()
        QTimer.singleShot(0, lambda: self.scroll.horizontalScrollBar().setValue(pos))
        self.dirty()

    def load_example(self):
        self.runner_stop_if("pattern")
        self.state["pattern"]["steps"] = example_steps()
        self.refresh_timeline()
        self.dirty()

    def clear_steps(self):
        if not self.state["pattern"]["steps"]:
            return
        if QMessageBox.question(self, APP_NAME, tr("clear_q")) == QMessageBox.Yes:
            self.runner_stop_if("pattern")
            self.state["pattern"]["steps"] = []
            self.refresh_timeline()
            self.dirty()

    def save_preset(self):
        path, _ = QFileDialog.getSaveFileName(self, tr("save"), "pattern.json", "AutoKey (*.json)")
        if path:
            with open(path, "w", encoding="utf-8") as fh:
                json.dump({"autokey": 2, "pattern": self.state["pattern"]}, fh, indent=2, ensure_ascii=False)

    def load_preset(self):
        path, _ = QFileDialog.getOpenFileName(self, tr("open"), "", "AutoKey (*.json)")
        if not path:
            return
        try:
            with open(path, encoding="utf-8") as fh:
                data = json.load(fh)
            pat = data["pattern"]
            for s in pat["steps"]:
                assert s["type"] in KIND_COLOR
            self.state["pattern"].update(pat)
        except Exception:
            QMessageBox.warning(self, APP_NAME, tr("bad_file"))
            return
        self.runner_stop_if("pattern")
        self.build()
        self.dirty()

    # ---- réglages fenêtre
    def _drag(self, ev):
        if ev.button() == Qt.LeftButton and self.windowHandle():
            self.windowHandle().startSystemMove()

    def apply_size(self):
        self.setFixedSize(1120, 420 if self.state["mode"] == "spam" else 530)

    def set_mode(self, m):
        self.state["mode"] = m
        self.stack.setCurrentIndex(0 if m == "spam" else 1)
        self.apply_size()
        self.dirty()

    def set_lang(self, code):
        global _lang
        self.cancel_record()
        self.state["lang"] = _lang = code
        self.build()
        self.dirty()

    def set_pin(self, on):
        self.state["pin"] = on
        self.setWindowFlag(Qt.WindowStaysOnTopHint, on)
        self.show()
        self.dirty()

    # ---- démarrage / arrêt
    def toggle(self, which):
        if self.runner.running and self.runner.mode == which:
            self.runner.stop()
        else:
            self.start(which)

    def runner_stop_if(self, which):
        if self.runner.running and self.runner.mode == which:
            self.runner.stop_and_wait()

    def start(self, which):
        if self.runner.running and self.runner.mode == which:
            return
        if which == "spam":
            st = self.state["spam"]
            iv = max(0.001, secs(st["value"], st["unit"]))
            self.runner.start("spam", job_spam, to_pynput(st["key"]), iv)
        else:
            st = self.state["pattern"]
            if not st["steps"]:
                self.status.setText(tr("no_steps"))
                return
            compiled = []
            for s in st["steps"]:
                c = {"type": s["type"]}
                if "key" in s:
                    c["k"] = to_pynput(s["key"])
                if s["type"] == "press":
                    c["count"], c["gap"] = int(s["count"]), s["gap"] / 1000
                else:
                    c["sec"] = secs(s["value"], s["unit"])
                compiled.append(c)
            self.runner.start("pattern", job_pattern, compiled, int(st["loops"]))

    # ---- capture de touches
    def begin_record(self, cap):
        if self.recorder is cap:
            return self.cancel_record()
        self.cancel_record()
        self.recorder = cap
        self.rec_keys, self.rec_down = [], set()
        cap.set_recording(True)

    def cancel_record(self):
        if self.recorder is not None:
            try:
                self.recorder.set_recording(False)
            except RuntimeError:
                pass
            self.recorder = None

    def on_key_down(self, spec):
        if self.recorder is not None:
            if spec["id"] not in {s["id"] for s in self.rec_keys}:
                self.rec_keys.append(spec)
            self.rec_down.add(spec["id"])
            if not self.recorder.combo:
                cap, self.recorder = self.recorder, None
                cap.finish([spec])
            else:
                self.recorder.refresh(combo_sort(self.rec_keys))
            return
        self.pressed.add(spec["id"])
        self.check_bindings()

    def on_key_up(self, spec):
        if self.recorder is not None and self.recorder.combo:
            self.rec_down.discard(spec["id"])
            if self.rec_keys and not self.rec_down:
                cap, self.recorder = self.recorder, None
                cap.finish(combo_sort(self.rec_keys))
            return
        self.pressed.discard(spec["id"])
        self.check_bindings()

    def check_bindings(self):
        for which in ("spam", "pattern"):
            ids = {s["id"] for s in self.state[which]["hotkey"]}
            active = bool(ids) and ids <= self.pressed
            if active == self.bind_active[which]:
                continue
            self.bind_active[which] = active
            if self.state[which]["hk_mode"] == "toggle":
                if active:
                    self.toggle(which)
            elif active:
                self.start(which)
            else:
                self.runner_stop_if(which)

    # ---- rafraîchissement
    def poll(self):
        r = self.runner
        run = r.running
        self.dot.tick(run)
        self.start_spam.set_running(run and r.mode == "spam")
        self.start_pat.set_running(run and r.mode == "pattern")
        if run and r.mode == "spam":
            self.status.setText(tr("running_spam", n=r.count))
        elif run and r.mode == "pattern" and r.step >= 0:
            self.status.setText(tr("running_pat", i=r.loop, n=r.loops or "∞", s=r.step + 1, t=r.steps_total))
        elif not run and self.status.text() != tr("no_steps"):
            self.status.setText(tr("idle"))
        active = r.step if run and r.mode == "pattern" else -1
        for i, c in enumerate(self.step_cards):
            c.set_active(i == active)

    def closeEvent(self, ev):
        self.runner.stop_and_wait()
        self.listener.stop()
        save_state(self.state)
        ev.accept()


def write_icon(path):
    app = QApplication.instance() or QApplication(sys.argv)  # noqa: F841
    img = logo_pixmap(256).toImage()
    ok = img.save(path)
    print("icon:", path, ok)
    return ok


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "--make-icon":
        sys.exit(0 if write_icon(sys.argv[2]) else 1)
    if IS_WIN:
        try:  # icône propre dans la barre des tâches
            import ctypes
            ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID("AutoKey.App")
        except Exception:
            pass
    app = QApplication(sys.argv)
    app.setStyleSheet(QSS)
    w = MainWindow()
    w.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
