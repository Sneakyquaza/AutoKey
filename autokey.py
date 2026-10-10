"""
AutoKey — auto-presser clavier avec spam simple et patterns en boucle.
Dépendance : pip install pynput
"""
import json
import queue
import threading
import tkinter as tk
from tkinter import ttk, messagebox, filedialog

from pynput import keyboard
from pynput.keyboard import Key, KeyCode, Controller

kb = Controller()
UNITS = {"ms": 0.001, "s": 1.0, "min": 60.0}
TYPES = {"Maintenir": "hold", "Appuyer N fois": "press", "Attendre": "wait"}
TYPES_INV = {v: k for k, v in TYPES.items()}
ALIASES = {
    "espace": "space", "entree": "enter", "entrée": "enter", "echap": "esc",
    "maj": "shift", "haut": "up", "bas": "down", "gauche": "left", "droite": "right",
    "suppr": "delete", "retour": "backspace",
}


def parse_key(s: str):
    s = s.strip()
    if not s:
        raise ValueError("Touche vide")
    if len(s) == 1:
        return KeyCode.from_char(s.lower())
    name = ALIASES.get(s.lower(), s.lower())
    try:
        return Key[name]
    except KeyError:
        raise ValueError(f"Touche inconnue : {s}")


def to_float(s: str) -> float:
    v = float(str(s).replace(",", "."))
    if v < 0:
        raise ValueError("Valeur négative")
    return v


# ---------------------------------------------------------------- moteur
class Runner:
    def __init__(self):
        self.thread = None
        self.stop_evt = threading.Event()
        self.held = set()
        self.mode = ""
        self.info = ""

    @property
    def running(self):
        return self.thread is not None and self.thread.is_alive()

    def start(self, mode, target, *args):
        if self.running:
            return
        self.stop_evt.clear()
        self.mode, self.info = mode, ""
        self.thread = threading.Thread(target=self._wrap, args=(target, *args), daemon=True)
        self.thread.start()

    def _wrap(self, target, *args):
        try:
            target(self, *args)
        finally:
            for k in list(self.held):  # ne jamais laisser une touche bloquée
                kb.release(k)
            self.held.clear()

    def stop(self):
        self.stop_evt.set()

    def wait(self, sec):
        """Attente interruptible. Renvoie False si stop demandé."""
        return not self.stop_evt.wait(sec)

    def tap(self, k):
        kb.press(k)
        kb.release(k)

    def hold(self, k, sec):
        kb.press(k)
        self.held.add(k)
        ok = self.wait(sec)
        kb.release(k)
        self.held.discard(k)
        return ok


def job_spam(r: Runner, key, interval):
    n = 0
    while not r.stop_evt.is_set():
        r.tap(key)
        n += 1
        r.info = f"{n} appuis"
        if not r.wait(interval):
            break


def job_pattern(r: Runner, steps, loops):
    i = 0
    while loops == 0 or i < loops:
        i += 1
        for n, s in enumerate(steps, 1):
            r.info = f"Boucle {i}/{loops or '∞'} — étape {n}/{len(steps)}"
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


# ---------------------------------------------------------------- GUI
class App:
    def __init__(self, root):
        self.root = root
        root.title("AutoKey")
        root.resizable(False, False)
        self.runner = Runner()
        self.q = queue.Queue()
        self.hk = None
        self.steps = []

        nb = ttk.Notebook(root)
        nb.pack(fill="both", expand=True, padx=8, pady=8)
        self._build_spam(nb)
        self._build_pattern(nb)
        self._build_bottom()

        self.apply_hotkeys()
        root.protocol("WM_DELETE_WINDOW", self.on_close)
        self._poll()

    # ---- onglet spam
    def _build_spam(self, nb):
        f = ttk.Frame(nb, padding=10)
        nb.add(f, text="Spam simple")
        self.spam_key = tk.StringVar(value="e")
        self.spam_val = tk.StringVar(value="100")
        self.spam_unit = tk.StringVar(value="ms")
        self.hk_spam = tk.StringVar(value="<f6>")

        ttk.Label(f, text="Touche :").grid(row=0, column=0, sticky="w")
        ttk.Entry(f, textvariable=self.spam_key, width=12).grid(row=0, column=1, sticky="w")
        ttk.Label(f, text="Toutes les :").grid(row=1, column=0, sticky="w", pady=4)
        ttk.Entry(f, textvariable=self.spam_val, width=12).grid(row=1, column=1, sticky="w")
        ttk.Combobox(f, textvariable=self.spam_unit, values=list(UNITS), width=5,
                     state="readonly").grid(row=1, column=2, sticky="w", padx=4)
        ttk.Label(f, text="Raccourci on/off :").grid(row=2, column=0, sticky="w")
        ttk.Entry(f, textvariable=self.hk_spam, width=20).grid(row=2, column=1, columnspan=2, sticky="w")
        self.btn_spam = ttk.Button(f, text="Démarrer", command=self.toggle_spam)
        self.btn_spam.grid(row=3, column=0, columnspan=3, sticky="ew", pady=10)

    # ---- onglet pattern
    def _build_pattern(self, nb):
        f = ttk.Frame(nb, padding=10)
        nb.add(f, text="Pattern")

        self.tree = ttk.Treeview(f, columns=("n", "action", "key", "detail"),
                                 show="headings", height=8, selectmode="browse")
        for c, t, w in (("n", "#", 30), ("action", "Action", 110),
                        ("key", "Touche", 70), ("detail", "Détail", 130)):
            self.tree.heading(c, text=t)
            self.tree.column(c, width=w, anchor="w")
        self.tree.grid(row=0, column=0, columnspan=6, sticky="ew")
        self.tree.bind("<<TreeviewSelect>>", self.on_select)

        # formulaire d'étape
        self.f_type = tk.StringVar(value="Maintenir")
        self.f_key = tk.StringVar(value="z")
        self.f_dur = tk.StringVar(value="3")
        self.f_unit = tk.StringVar(value="s")
        self.f_count = tk.StringVar(value="10")
        self.f_gap = tk.StringVar(value="100")

        form = ttk.LabelFrame(f, text="Étape", padding=6)
        form.grid(row=1, column=0, columnspan=6, sticky="ew", pady=6)
        ttk.Combobox(form, textvariable=self.f_type, values=list(TYPES), width=14,
                     state="readonly").grid(row=0, column=0, padx=2)
        ttk.Label(form, text="Touche").grid(row=0, column=1)
        ttk.Entry(form, textvariable=self.f_key, width=8).grid(row=0, column=2, padx=2)
        ttk.Label(form, text="Durée").grid(row=1, column=1)
        ttk.Entry(form, textvariable=self.f_dur, width=8).grid(row=1, column=2, padx=2)
        ttk.Combobox(form, textvariable=self.f_unit, values=list(UNITS), width=5,
                     state="readonly").grid(row=1, column=3, padx=2)
        ttk.Label(form, text="Nombre").grid(row=0, column=3)
        ttk.Entry(form, textvariable=self.f_count, width=6).grid(row=0, column=4, padx=2)
        ttk.Label(form, text="Écart (ms)").grid(row=1, column=4)
        ttk.Entry(form, textvariable=self.f_gap, width=6).grid(row=1, column=5, padx=2)
        ttk.Label(form, text="Durée → Maintenir / Attendre · Nombre + Écart → Appuyer N fois",
                  foreground="gray").grid(row=2, column=0, columnspan=6, sticky="w", pady=(4, 0))

        btns = ttk.Frame(f)
        btns.grid(row=2, column=0, columnspan=6, sticky="ew")
        for txt, cmd in (("Ajouter", self.add_step), ("Modifier", self.edit_step),
                         ("Supprimer", self.del_step), ("▲", lambda: self.move(-1)),
                         ("▼", lambda: self.move(1))):
            ttk.Button(btns, text=txt, command=cmd, width=9 if len(txt) > 1 else 3).pack(side="left", padx=1)

        opts = ttk.Frame(f)
        opts.grid(row=3, column=0, columnspan=6, sticky="ew", pady=8)
        self.loops = tk.StringVar(value="0")
        self.hk_pattern = tk.StringVar(value="<f7>")
        ttk.Label(opts, text="Boucles (0 = infini) :").grid(row=0, column=0, sticky="w")
        ttk.Spinbox(opts, from_=0, to=100000, textvariable=self.loops, width=8).grid(row=0, column=1, sticky="w")
        ttk.Label(opts, text="Raccourci on/off :").grid(row=1, column=0, sticky="w", pady=4)
        ttk.Entry(opts, textvariable=self.hk_pattern, width=20).grid(row=1, column=1, sticky="w")
        ttk.Button(opts, text="Sauver…", command=self.save).grid(row=0, column=2, padx=6)
        ttk.Button(opts, text="Charger…", command=self.load).grid(row=1, column=2, padx=6)

        self.btn_pattern = ttk.Button(f, text="Démarrer", command=self.toggle_pattern)
        self.btn_pattern.grid(row=4, column=0, columnspan=6, sticky="ew")

    # ---- bas de fenêtre
    def _build_bottom(self):
        f = ttk.Frame(self.root, padding=(8, 0, 8, 8))
        f.pack(fill="x")
        self.hk_stop = tk.StringVar(value="<f8>")
        ttk.Label(f, text="Stop d'urgence :").grid(row=0, column=0, sticky="w")
        ttk.Entry(f, textvariable=self.hk_stop, width=14).grid(row=0, column=1, sticky="w")
        ttk.Button(f, text="Appliquer raccourcis", command=self.apply_hotkeys).grid(row=0, column=2, padx=6)
        ttk.Label(f, text="Format : <f6>  <ctrl>+<alt>+p  <shift>+x", foreground="gray").grid(
            row=1, column=0, columnspan=3, sticky="w")
        self.status = tk.StringVar(value="■ Arrêté")
        ttk.Label(f, textvariable=self.status, font=("Segoe UI", 10, "bold")).grid(
            row=2, column=0, columnspan=3, sticky="w", pady=(6, 0))

    # ---- étapes
    @staticmethod
    def detail(s):
        if s["type"] == "press":
            return f"{s['count']}× (écart {s['gap_ms']:g} ms)"
        return f"{s['duration']:g} {s['unit']}"

    def refresh_tree(self, select=None):
        self.tree.delete(*self.tree.get_children())
        for i, s in enumerate(self.steps):
            self.tree.insert("", "end", iid=str(i), values=(
                i + 1, TYPES_INV[s["type"]], s.get("key", "—"), self.detail(s)))
        if select is not None and 0 <= select < len(self.steps):
            self.tree.selection_set(str(select))

    def sel_index(self):
        sel = self.tree.selection()
        return int(sel[0]) if sel else None

    def form_step(self):
        t = TYPES[self.f_type.get()]
        s = {"type": t}
        if t != "wait":
            parse_key(self.f_key.get())
            s["key"] = self.f_key.get().strip()
        if t in ("hold", "wait"):
            s["duration"] = to_float(self.f_dur.get())
            s["unit"] = self.f_unit.get()
        if t == "press":
            s["count"] = max(1, int(self.f_count.get()))
            s["gap_ms"] = to_float(self.f_gap.get())
        return s

    def add_step(self):
        try:
            self.steps.append(self.form_step())
        except ValueError as e:
            return messagebox.showerror("Erreur", str(e))
        self.refresh_tree(len(self.steps) - 1)

    def edit_step(self):
        i = self.sel_index()
        if i is None:
            return
        try:
            self.steps[i] = self.form_step()
        except ValueError as e:
            return messagebox.showerror("Erreur", str(e))
        self.refresh_tree(i)

    def del_step(self):
        i = self.sel_index()
        if i is not None:
            del self.steps[i]
            self.refresh_tree(min(i, len(self.steps) - 1))

    def move(self, d):
        i = self.sel_index()
        if i is None or not 0 <= i + d < len(self.steps):
            return
        self.steps[i], self.steps[i + d] = self.steps[i + d], self.steps[i]
        self.refresh_tree(i + d)

    def on_select(self, _):
        i = self.sel_index()
        if i is None:
            return
        s = self.steps[i]
        self.f_type.set(TYPES_INV[s["type"]])
        self.f_key.set(s.get("key", ""))
        if "duration" in s:
            self.f_dur.set(f"{s['duration']:g}")
            self.f_unit.set(s["unit"])
        if "count" in s:
            self.f_count.set(str(s["count"]))
            self.f_gap.set(f"{s['gap_ms']:g}")

    # ---- démarrage / arrêt
    def toggle_spam(self):
        if self.runner.running:
            return self.runner.stop()
        try:
            key = parse_key(self.spam_key.get())
            iv = max(0.001, to_float(self.spam_val.get()) * UNITS[self.spam_unit.get()])
        except ValueError as e:
            return messagebox.showerror("Erreur", str(e))
        self.runner.start("spam", job_spam, key, iv)

    def toggle_pattern(self):
        if self.runner.running:
            return self.runner.stop()
        if not self.steps:
            return messagebox.showwarning("Pattern", "Aucune étape.")
        try:
            loops = int(self.loops.get())
            compiled = []
            for s in self.steps:
                c = {"type": s["type"]}
                if "key" in s:
                    c["k"] = parse_key(s["key"])
                if "duration" in s:
                    c["sec"] = s["duration"] * UNITS[s["unit"]]
                if "count" in s:
                    c["count"], c["gap"] = s["count"], s["gap_ms"] / 1000
                compiled.append(c)
        except ValueError as e:
            return messagebox.showerror("Erreur", str(e))
        self.runner.start("pattern", job_pattern, compiled, max(0, loops))

    # ---- raccourcis globaux
    def apply_hotkeys(self):
        if self.hk:
            self.hk.stop()
            self.hk = None
        mapping = {}
        for var, action in ((self.hk_spam, "spam"), (self.hk_pattern, "pattern"), (self.hk_stop, "stop")):
            combo = var.get().strip()
            if combo:
                mapping[combo] = (lambda a=action: self.q.put(a))
        try:
            self.hk = keyboard.GlobalHotKeys(mapping)
            self.hk.start()
        except Exception as e:
            messagebox.showerror("Raccourcis", f"Raccourci invalide : {e}")

    # ---- sauvegarde
    def save(self):
        path = filedialog.asksaveasfilename(defaultextension=".json", filetypes=[("Pattern", "*.json")])
        if path:
            with open(path, "w", encoding="utf-8") as fh:
                json.dump({"steps": self.steps, "loops": self.loops.get(),
                           "hotkey": self.hk_pattern.get()}, fh, indent=2, ensure_ascii=False)

    def load(self):
        path = filedialog.askopenfilename(filetypes=[("Pattern", "*.json")])
        if not path:
            return
        try:
            with open(path, encoding="utf-8") as fh:
                data = json.load(fh)
            self.steps = data["steps"]
            self.loops.set(str(data.get("loops", 0)))
            if data.get("hotkey"):
                self.hk_pattern.set(data["hotkey"])
                self.apply_hotkeys()
        except Exception as e:
            return messagebox.showerror("Erreur", f"Fichier invalide : {e}")
        self.refresh_tree()

    # ---- boucle UI
    def _poll(self):
        while not self.q.empty():
            a = self.q.get_nowait()
            if a == "spam":
                self.toggle_spam()
            elif a == "pattern":
                self.toggle_pattern()
            else:
                self.runner.stop()
        run = self.runner.running
        mode = self.runner.mode if run else ""
        self.status.set(f"▶ {mode.upper()} — {self.runner.info}" if run else "■ Arrêté")
        self.btn_spam.config(text="Stop" if mode == "spam" else "Démarrer")
        self.btn_pattern.config(text="Stop" if mode == "pattern" else "Démarrer")
        self.root.after(50, self._poll)

    def on_close(self):
        self.runner.stop()
        if self.hk:
            self.hk.stop()
        self.root.destroy()


if __name__ == "__main__":
    root = tk.Tk()
    App(root)
    root.mainloop()
