//! Rendu logiciel d'egui (pas d'OpenGL/GPU) : rasteriseur de triangles + redessin partiel.
//! Seules les zones qui ont changé depuis l'image précédente sont redessinées.
use egui::epaint::{textures::TexturesDelta, ClippedPrimitive, ClippedShape, ColorMode, ImageData, Mesh, PathStroke, Primitive, Shape, Tessellator, TextureId};
use std::sync::Arc;
use egui::{Color32, Rect};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    fn empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }
    fn inter(&self, o: &IRect) -> IRect {
        IRect { x0: self.x0.max(o.x0), y0: self.y0.max(o.y0), x1: self.x1.min(o.x1), y1: self.y1.min(o.y1) }
    }
    fn union(&self, o: &IRect) -> IRect {
        IRect { x0: self.x0.min(o.x0), y0: self.y0.min(o.y0), x1: self.x1.max(o.x1), y1: self.y1.max(o.y1) }
    }
    fn overlaps(&self, o: &IRect) -> bool {
        !self.inter(o).empty()
    }
    fn area(&self) -> i64 {
        if self.empty() {
            0
        } else {
            (self.x1 - self.x0) as i64 * (self.y1 - self.y0) as i64
        }
    }
    fn from_rect(r: Rect, ppp: f32) -> IRect {
        IRect {
            x0: (r.min.x * ppp).floor() as i32,
            y0: (r.min.y * ppp).floor() as i32,
            x1: (r.max.x * ppp).ceil() as i32,
            y1: (r.max.y * ppp).ceil() as i32,
        }
    }
}

struct Tex {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>, // RGBA prémultiplié (espace gamma, comme egui)
}

struct Prim {
    hash: u64,
    bbox: IRect,
    clip: IRect,
    tex: TextureId,
    mesh: Arc<Mesh>,
}

/// Maillages déjà calculés pour une forme identique (évite de re-tesseller à chaque image)
struct Cached {
    prims: Vec<(IRect, IRect, TextureId, Arc<Mesh>)>,
    _keep: Option<Arc<egui::Galley>>, // garde le texte vivant : son adresse ne peut pas être réutilisée
}

struct H(u64);
impl H {
    #[inline]
    fn u(&mut self, v: u64) {
        self.0 ^= v;
        self.0 = self.0.wrapping_mul(0x0100_0000_01b3).rotate_left(23);
    }
    #[inline]
    fn f(&mut self, v: f32) {
        self.u(v.to_bits() as u64)
    }
    fn p(&mut self, p: egui::Pos2) {
        self.u(((p.x.to_bits() as u64) << 32) | p.y.to_bits() as u64)
    }
    fn c(&mut self, c: Color32) {
        self.u(u32::from_ne_bytes(c.to_array()) as u64)
    }
    fn r(&mut self, r: Rect) {
        self.p(r.min);
        self.p(r.max);
    }
    fn stroke(&mut self, s: &egui::Stroke) {
        self.f(s.width);
        self.c(s.color);
    }
    fn pstroke(&mut self, s: &PathStroke) -> bool {
        self.f(s.width);
        self.u(s.kind.clone() as u64);
        match &s.color {
            ColorMode::Solid(c) => {
                self.c(*c);
                true
            }
            _ => false,
        }
    }
    fn tex(&mut self, t: TextureId) {
        self.u(match t {
            TextureId::Managed(i) => i,
            TextureId::User(i) => i ^ 0xdead_beef,
        });
    }
}

/// Empreinte d'une forme (false = forme non gérée → toujours re-tessellée)
fn hash_shape(h: &mut H, s: &Shape, keep: &mut Option<Arc<egui::Galley>>) -> bool {
    match s {
        Shape::Noop => {
            h.u(1);
            true
        }
        Shape::Vec(v) => {
            h.u(2);
            v.iter().all(|x| hash_shape(h, x, keep))
        }
        Shape::Circle(c) => {
            h.u(3);
            h.p(c.center);
            h.f(c.radius);
            h.c(c.fill);
            h.stroke(&c.stroke);
            true
        }
        Shape::LineSegment { points, stroke } => {
            h.u(4);
            h.p(points[0]);
            h.p(points[1]);
            h.pstroke(stroke)
        }
        Shape::Path(ps) => {
            h.u(5);
            for pt in &ps.points {
                h.p(*pt);
            }
            h.u(ps.closed as u64);
            h.c(ps.fill);
            h.pstroke(&ps.stroke)
        }
        Shape::Rect(r) => {
            h.u(6);
            h.r(r.rect);
            h.f(r.rounding.nw);
            h.f(r.rounding.ne);
            h.f(r.rounding.sw);
            h.f(r.rounding.se);
            h.c(r.fill);
            h.stroke(&r.stroke);
            h.f(r.blur_width);
            h.tex(r.fill_texture_id);
            h.r(r.uv);
            true
        }
        Shape::Text(t) => {
            if keep.is_some() {
                return false; // un seul texte par forme gardé : sinon on recalcule
            }
            h.u(7);
            h.p(t.pos);
            h.u(Arc::as_ptr(&t.galley) as usize as u64);
            h.stroke(&t.underline);
            h.c(t.fallback_color);
            h.c(t.override_text_color.unwrap_or(Color32::TRANSPARENT));
            h.u(t.override_text_color.is_some() as u64);
            h.f(t.opacity_factor);
            h.f(t.angle);
            *keep = Some(t.galley.clone());
            true
        }
        Shape::Mesh(m) => {
            h.u(8);
            h.tex(m.texture_id);
            for v in &m.vertices {
                h.p(v.pos);
                h.p(v.uv);
                h.c(v.color);
            }
            for i in &m.indices {
                h.u(*i as u64);
            }
            true
        }
        _ => false,
    }
}

pub struct Renderer {
    pub w: usize,
    pub h: usize,
    pub fb: Vec<u32>, // 0x00RRGGBB
    bg: u32,
    textures: HashMap<TextureId, Tex>,
    prev: Vec<(u64, IRect)>,
    force_full: bool,
    cache: HashMap<u64, Vec<Cached>>,
}

fn pack(c: [u8; 4]) -> u32 {
    ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32
}

impl Renderer {
    pub fn new(bg: Color32) -> Self {
        Renderer {
            w: 0,
            h: 0,
            fb: vec![],
            bg: pack(bg.to_array()),
            textures: HashMap::new(),
            prev: vec![],
            force_full: true,
            cache: HashMap::new(),
        }
    }

    pub fn set_textures(&mut self, delta: &TexturesDelta) {
        for (id, d) in &delta.set {
            let (w, h, pixels): (usize, usize, Vec<[u8; 4]>) = match &d.image {
                ImageData::Color(img) => (img.width(), img.height(), img.pixels.iter().map(|c| c.to_array()).collect()),
                ImageData::Font(img) => (img.width(), img.height(), img.srgba_pixels(None).map(|c| c.to_array()).collect()),
            };
            match d.pos {
                None => {
                    self.textures.insert(*id, Tex { w, h, px: pixels });
                    self.force_full = true;
                }
                Some([x, y]) => {
                    if let Some(t) = self.textures.get_mut(id) {
                        for row in 0..h {
                            let dst = (y + row) * t.w + x;
                            if y + row < t.h && x + w <= t.w {
                                t.px[dst..dst + w].copy_from_slice(&pixels[row * w..row * w + w]);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn free_textures(&mut self, delta: &TexturesDelta) {
        for id in &delta.free {
            self.textures.remove(id);
        }
    }

    /// Dessine l'image ; renvoie les zones modifiées (en pixels).
    pub fn render(&mut self, ctx: &egui::Context, shapes: Vec<ClippedShape>, ppp: f32, w: usize, h: usize) -> Vec<IRect> {
        if w != self.w || h != self.h {
            self.w = w;
            self.h = h;
            self.fb = vec![self.bg; w * h];
            self.force_full = true;
        }
        let screen = IRect { x0: 0, y0: 0, x1: w as i32, y1: h as i32 };

        // 1) tessellation forme par forme (pour pouvoir comparer avec l'image précédente)
        let atlas = ctx.fonts(|f| f.texture_atlas());
        let font_tex_size = atlas.lock().size();
        // pas de disques pré-rendus : petits cercles en vrais polygones antialiasés (plus nets en logiciel)
        let mut tess = Tessellator::new(ppp, ctx.tessellation_options(|o| *o), font_tex_size, vec![]);
        let mut prims: Vec<Prim> = Vec::with_capacity(shapes.len());
        let mut out: Vec<ClippedPrimitive> = Vec::with_capacity(2);
        let mut old_cache = std::mem::take(&mut self.cache);
        for cs in shapes {
            // empreinte de la forme + de tout ce qui influence sa tessellation
            let mut h = H(0xcbf2_9ce4_8422_2325);
            h.f(ppp);
            h.u(((font_tex_size[0] as u64) << 32) | font_tex_size[1] as u64);
            h.r(cs.clip_rect);
            let mut keep = None;
            let hashable = hash_shape(&mut h, &cs.shape, &mut keep);
            let key = h.0;
            let hit = if hashable { old_cache.get_mut(&key).and_then(|v| v.pop()) } else { None };
            let entry = match hit {
                Some(c) => c,
                None => {
                    out.clear();
                    tess.tessellate_clipped_shape(cs, &mut out);
                    let mut list = vec![];
                    for cp in out.drain(..) {
                        if let Primitive::Mesh(mesh) = cp.primitive {
                            if mesh.indices.is_empty() {
                                continue;
                            }
                            let clip = IRect::from_rect(cp.clip_rect, ppp).inter(&screen);
                            let b = mesh.calc_bounds();
                            let bbox = IRect {
                                x0: (b.min.x * ppp).floor() as i32 - 1,
                                y0: (b.min.y * ppp).floor() as i32 - 1,
                                x1: (b.max.x * ppp).ceil() as i32 + 1,
                                y1: (b.max.y * ppp).ceil() as i32 + 1,
                            }
                            .inter(&clip);
                            if bbox.empty() {
                                continue;
                            }
                            list.push((bbox, clip, mesh.texture_id, Arc::new(mesh)));
                        }
                    }
                    Cached { prims: list, _keep: keep }
                }
            };
            for (i, (bbox, clip, tex, mesh)) in entry.prims.iter().enumerate() {
                // non hachable → empreinte unique : toujours considéré comme modifié
                let hash = if hashable { key ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) } else { self.prev.len() as u64 ^ 0xF00D ^ (prims.len() as u64) << 20 };
                prims.push(Prim { hash, bbox: *bbox, clip: *clip, tex: *tex, mesh: mesh.clone() });
            }
            if hashable {
                self.cache.entry(key).or_default().push(entry);
            }
        }
        drop(old_cache);

        // 2) zones modifiées = formes apparues ou disparues
        let mut damage: Vec<IRect> = vec![];
        if self.force_full {
            damage.push(screen);
        } else {
            let mut old: HashMap<u64, Vec<IRect>> = HashMap::new();
            for (hsh, b) in &self.prev {
                old.entry(*hsh).or_default().push(*b);
            }
            for p in &prims {
                match old.get_mut(&p.hash).and_then(|v| v.pop()) {
                    Some(_) => {}
                    None => damage.push(p.bbox),
                }
            }
            for v in old.values() {
                damage.extend(v.iter().copied());
            }
            damage = merge(damage, &screen);
        }
        self.force_full = false;
        self.prev = prims.iter().map(|p| (p.hash, p.bbox)).collect();

        // 3) on ne redessine que ces zones
        for d in &damage {
            let d = d.inter(&screen);
            if d.empty() {
                continue;
            }
            for y in d.y0..d.y1 {
                let row = y as usize * w;
                self.fb[row + d.x0 as usize..row + d.x1 as usize].fill(self.bg);
            }
            for p in &prims {
                if !p.bbox.overlaps(&d) {
                    continue;
                }
                let clip = p.clip.inter(&d);
                let tex = self.textures.get(&p.tex);
                raster_mesh(&mut self.fb, w, &p.mesh, tex, ppp, clip);
            }
        }
        #[cfg(feature = "shots")]
        {
            // contrôle : l'image partielle doit être identique à un rendu complet
            let mut full = vec![self.bg; w * h];
            for p in &prims {
                raster_mesh(&mut full, w, &p.mesh, self.textures.get(&p.tex), ppp, p.clip);
            }
            let diff = full.iter().zip(&self.fb).filter(|(a, b)| a != b).count();
            if diff > 0 {
                eprintln!("DAMAGE MISMATCH: {diff} px");
            }
        }
        damage
    }
}

/// Fusionne les zones qui se chevauchent ; trop de zones → une seule englobante.
fn merge(mut v: Vec<IRect>, screen: &IRect) -> Vec<IRect> {
    v.retain(|r| !r.inter(screen).empty());
    let mut changed = true;
    while changed {
        changed = false;
        'outer: for i in 0..v.len() {
            for j in i + 1..v.len() {
                let a = v[i];
                let b = v[j];
                let grown = IRect { x0: a.x0 - 4, y0: a.y0 - 4, x1: a.x1 + 4, y1: a.y1 + 4 };
                if grown.overlaps(&b) {
                    v[i] = a.union(&b);
                    v.swap_remove(j);
                    changed = true;
                    break 'outer;
                }
            }
        }
    }
    if v.len() > 12 {
        let u = v.iter().skip(1).fold(v[0], |a, b| a.union(b));
        return vec![u.inter(screen)];
    }
    let total: i64 = v.iter().map(|r| r.area()).sum();
    if total > screen.area() * 6 / 10 {
        return vec![*screen];
    }
    v.into_iter().map(|r| r.inter(screen)).collect()
}

#[inline]
fn blend(dst: u32, s: [u32; 4]) -> u32 {
    let a = s[3];
    if a >= 255 {
        return (s[0] << 16) | (s[1] << 8) | s[2];
    }
    let inv = 255 - a;
    let dr = (dst >> 16) & 0xFF;
    let dg = (dst >> 8) & 0xFF;
    let db = dst & 0xFF;
    let r = (s[0] + (dr * inv + 127) / 255).min(255);
    let g = (s[1] + (dg * inv + 127) / 255).min(255);
    let b = (s[2] + (db * inv + 127) / 255).min(255);
    (r << 16) | (g << 8) | b
}

#[inline]
fn sample(t: &Tex, u: f32, v: f32) -> [u8; 4] {
    let x = ((u * t.w as f32) as isize).clamp(0, t.w as isize - 1) as usize;
    let y = ((v * t.h as f32) as isize).clamp(0, t.h as isize - 1) as usize;
    t.px[y * t.w + x]
}

#[inline]
fn modulate(c: [u8; 4], t: [u8; 4]) -> [u32; 4] {
    [
        (c[0] as u32 * t[0] as u32 + 127) / 255,
        (c[1] as u32 * t[1] as u32 + 127) / 255,
        (c[2] as u32 * t[2] as u32 + 127) / 255,
        (c[3] as u32 * t[3] as u32 + 127) / 255,
    ]
}

fn raster_mesh(fb: &mut [u32], fbw: usize, mesh: &Mesh, tex: Option<&Tex>, ppp: f32, clip: IRect) {
    if clip.empty() {
        return;
    }
    let white = [255u8; 4];
    for tri in mesh.indices.chunks_exact(3) {
        let mut vs = [&mesh.vertices[tri[0] as usize], &mesh.vertices[tri[1] as usize], &mesh.vertices[tri[2] as usize]];
        let mut p = [vs[0].pos * ppp, vs[1].pos * ppp, vs[2].pos * ppp];
        let mut area = (p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[1].y - p[0].y) * (p[2].x - p[0].x);
        if area.abs() < 1e-6 {
            continue;
        }
        if area < 0.0 {
            vs.swap(1, 2);
            p.swap(1, 2);
            area = -area;
        }
        let x0 = (p[0].x.min(p[1].x).min(p[2].x).floor() as i32).max(clip.x0);
        let x1 = (p[0].x.max(p[1].x).max(p[2].x).ceil() as i32).min(clip.x1);
        let y0 = (p[0].y.min(p[1].y).min(p[2].y).floor() as i32).max(clip.y0);
        let y1 = (p[0].y.max(p[1].y).max(p[2].y).ceil() as i32).min(clip.y1);
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        let inv_area = 1.0 / area;
        // fonctions d'arête : w_i = A_i*x + B_i*y + C_i  (w0 opposé à p0, etc.)
        let edge = |a: egui::Pos2, b: egui::Pos2| (a.y - b.y, b.x - a.x, a.x * b.y - a.y * b.x);
        let e = [edge(p[1], p[2]), edge(p[2], p[0]), edge(p[0], p[1])];
        // règle haut-gauche : évite de dessiner deux fois les pixels des arêtes partagées
        let tl = |(a, b, _): (f32, f32, f32)| a > 0.0 || (a == 0.0 && b < 0.0);
        let bias = [if tl(e[0]) { 0.0 } else { -1e-5 }, if tl(e[1]) { 0.0 } else { -1e-5 }, if tl(e[2]) { 0.0 } else { -1e-5 }];

        let c = [vs[0].color.to_array(), vs[1].color.to_array(), vs[2].color.to_array()];
        let same_color = c[0] == c[1] && c[1] == c[2];
        let same_uv = vs[0].uv == vs[1].uv && vs[1].uv == vs[2].uv;
        let const_tex = if same_uv { tex.map(|t| sample(t, vs[0].uv.x, vs[0].uv.y)).unwrap_or(white) } else { white };
        let const_src = modulate(c[0], const_tex);
        if same_color && same_uv && const_src[3] == 0 && const_src[0] == 0 && const_src[1] == 0 && const_src[2] == 0 {
            continue; // totalement transparent
        }

        // plans d'interpolation (couleur, uv) : valeur = base + dx*x + dy*y  (calculés une fois par triangle)
        let plane = |f0: f32, f1: f32, f2: f32| -> (f32, f32, f32) {
            // f = (w0*f0 + w1*f1 + w2*f2)/area ; w_i = A_i x + B_i y + C_i
            let dx = (e[0].0 * f0 + e[1].0 * f1 + e[2].0 * f2) * inv_area;
            let dy = (e[0].1 * f0 + e[1].1 * f1 + e[2].1 * f2) * inv_area;
            let c0 = (e[0].2 * f0 + e[1].2 * f1 + e[2].2 * f2) * inv_area;
            (dx, dy, c0)
        };
        let interp_color = !same_color;
        let interp_uv = !same_uv && tex.is_some();
        let cp: [(f32, f32, f32); 4] = if interp_color {
            [0, 1, 2, 3].map(|i| plane(c[0][i] as f32, c[1][i] as f32, c[2][i] as f32))
        } else {
            [(0.0, 0.0, 0.0); 4]
        };
        let (up, vp) = if interp_uv {
            (plane(vs[0].uv.x, vs[1].uv.x, vs[2].uv.x), plane(vs[0].uv.y, vs[1].uv.y, vs[2].uv.y))
        } else {
            ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
        };

        for y in y0..y1 {
            let py = y as f32 + 0.5;
            let row = y as usize * fbw;
            // plage [xs, xe] où les 3 demi-plans sont positifs, calculée analytiquement
            let mut xs = x0;
            let mut xe = x1 - 1;
            let mut empty = false;
            for k in 0..3 {
                let (a, b, cc) = e[k];
                let val = b * py + cc + bias[k];
                if a > 1e-12 {
                    let lim = (-val / a - 0.5).ceil() as i32 - 1; // -1 : marge, testée ensuite
                    xs = xs.max(lim);
                } else if a < -1e-12 {
                    let lim = (-val / a - 0.5).floor() as i32 + 1;
                    xe = xe.min(lim);
                } else if val < 0.0 {
                    empty = true;
                }
            }
            if empty || xs > xe {
                continue;
            }
            let fx = xs as f32 + 0.5;
            let mut w = [
                e[0].0 * fx + e[0].1 * py + e[0].2 + bias[0],
                e[1].0 * fx + e[1].1 * py + e[1].2 + bias[1],
                e[2].0 * fx + e[2].1 * py + e[2].2 + bias[2],
            ];
            let mut col = [0f32; 4];
            if interp_color {
                for i in 0..4 {
                    col[i] = cp[i].0 * fx + cp[i].1 * py + cp[i].2 + 0.5;
                }
            }
            let mut uu = up.0 * fx + up.1 * py + up.2;
            let mut vv = vp.0 * fx + vp.1 * py + vp.2;
            for x in xs..=xe {
                if w[0] >= 0.0 && w[1] >= 0.0 && w[2] >= 0.0 {
                    let src = if !interp_color && !interp_uv {
                        const_src
                    } else {
                        let cc = if interp_color { [col[0] as u8, col[1] as u8, col[2] as u8, col[3] as u8] } else { c[0] };
                        let t = if interp_uv { sample(tex.unwrap(), uu, vv) } else { const_tex };
                        modulate(cc, t)
                    };
                    if src[3] != 0 || src[0] != 0 || src[1] != 0 || src[2] != 0 {
                        let i = row + x as usize;
                        fb[i] = blend(fb[i], src);
                    }
                }
                w[0] += e[0].0;
                w[1] += e[1].0;
                w[2] += e[2].0;
                if interp_color {
                    for i in 0..4 {
                        col[i] += cp[i].0;
                    }
                }
                if interp_uv {
                    uu += up.0;
                    vv += vp.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_rects() {
        let s = IRect { x0: 0, y0: 0, x1: 1000, y1: 500 };
        let v = merge(vec![IRect { x0: 0, y0: 0, x1: 10, y1: 10 }, IRect { x0: 8, y0: 8, x1: 20, y1: 20 }, IRect { x0: 500, y0: 300, x1: 510, y1: 310 }], &s);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn blend_premultiplied() {
        assert_eq!(blend(0x000000, [255, 0, 0, 255]), 0xFF0000);
        assert_eq!(blend(0xFFFFFF, [0, 0, 0, 0]), 0xFFFFFF);
        let half = blend(0x000000, [128, 128, 128, 128]);
        assert_eq!(half, 0x808080);
    }
}
