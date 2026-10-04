//! アトラス化したファイルのメッシュのテクスチャと UV を、今の編集のテクスチャに戻す。

use std::collections::{BTreeMap, BTreeSet};

use ugoe_core::Vec2;
use ugoe_format::ModuleFile;
use ugoe_model::Module;

use crate::atlas::{ATLAS_NAME_PREFIX, Placement};
use crate::texture::{Textures, decode_rgba_png};

/// 比べる側のファイル。テクスチャがアトラス化してあれば、メッシュのテクスチャと UV を今の編集のテクスチャに戻したもの。
pub(super) struct FileView<'a> {
    pub(super) file: &'a ModuleFile,
    /// ファイルのモジュール。戻せたメッシュは今の編集のテクスチャを指し、そのテクスチャは `textures` の後ろに足してある。
    pub(super) module: Module,
    /// `module.textures` の各番号の画像。
    pub(super) images: Vec<FileImage>,
    /// アトラスの中で、今の編集のテクスチャに戻せなかったメッシュがあるか (そのアトラスはテクスチャとして残る)。
    pub(super) unresolved: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum FileImage {
    /// ファイルの番号 `.0` のテクスチャ (と法線マップ)。
    File(usize),
    /// アトラスから見つけた今の編集のテクスチャ。`same` は画素が同じか。
    Packed { same: bool },
    /// 使うメッシュをすべて今の編集のテクスチャに戻したアトラス (テクスチャとしては比べない)。
    Hidden,
}

impl<'a> FileView<'a> {
    pub(super) fn new(file: &'a ModuleFile, module: &Module, images: &Textures) -> Self {
        let old = &file.module;
        let mut view = Self {
            file,
            module: old.clone(),
            images: (0..old.textures.len()).map(FileImage::File).collect(),
            unresolved: false,
        };
        // 今の編集に同じ名前のテクスチャがあれば、アトラスではなくそのテクスチャとして比べる。
        let atlases: BTreeSet<usize> = old
            .textures
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                t.name.starts_with(ATLAS_NAME_PREFIX)
                    && t.normal_map.is_none()
                    && !module.textures.iter().any(|n| n.name == t.name)
            })
            .map(|(i, _)| i)
            .collect();
        if atlases.is_empty() {
            return view;
        }
        // 戻し先の候補は、ファイルにアトラスでないテクスチャとして同じ名前が無い今の編集のテクスチャ。
        let candidates: Vec<usize> = (0..module.textures.len())
            .filter(|&t| {
                !old.textures
                    .iter()
                    .any(|o| o.name == module.textures[t].name)
            })
            .collect();
        let mut finder = Finder::new(file, module, images);
        let found = finder.resolve(&atlases, &candidates);

        // 戻せたテクスチャを後ろに足す。
        let mut index: BTreeMap<usize, u32> = BTreeMap::new();
        let mut same: BTreeMap<usize, bool> = BTreeMap::new();
        for &(t, _, place_same) in found.iter().flatten() {
            *same.entry(t).or_insert(true) &= place_same;
        }
        for (&t, &same) in &same {
            index.insert(
                t,
                u32::try_from(view.module.textures.len()).unwrap_or(u32::MAX),
            );
            view.module.textures.push(module.textures[t].clone());
            view.images.push(FileImage::Packed { same });
        }
        // アトラスを使うメッシュのうち、戻せなかったものが無いアトラスは隠す。
        for &a in &atlases {
            let on_atlas = |k: &usize| old.meshes[*k].texture as usize == a;
            let meshes: Vec<usize> = (0..old.meshes.len()).filter(on_atlas).collect();
            if meshes.iter().all(|&k| found[k].is_some()) && !meshes.is_empty() {
                view.images[a] = FileImage::Hidden;
            } else if !meshes.is_empty() {
                view.unresolved = true;
            }
        }
        for (mesh, found) in view.module.meshes.iter_mut().zip(&found) {
            let Some((t, place, _)) = found else {
                continue;
            };
            let current = module.meshes.iter().find(|m| m.id == mesh.id);
            mesh.texture = index[t];
            for (i, uv) in mesh.uvs.iter_mut().enumerate() {
                // 今の編集の UV を写すと同じ値になるなら、その UV (戻したときの丸めのずれを残さない)。
                *uv = match current.and_then(|c| c.uvs.get(i)) {
                    Some(&c) if place.map(c) == *uv => c,
                    _ => place.unmap(*uv),
                };
            }
        }
        view
    }
}

/// 画素を RGBA8 に広げた画像。
struct Rgba {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Rgba {
    fn decode(png: &[u8]) -> Option<Self> {
        let (width, height, pixels) = decode_rgba_png(png)?;
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    fn pixel(&self, x: u32, y: u32) -> &[u8] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        &self.pixels[i..i + 4]
    }

    /// `(x, y)` に `image` と同じ画素が並んでいるか。先に `samples` の画素だけを見る。
    fn contains_at(&self, image: &Self, x: u32, y: u32, samples: &[(u32, u32)]) -> bool {
        if x + image.width > self.width || y + image.height > self.height {
            return false;
        }
        let row = image.width as usize * 4;
        samples
            .iter()
            .all(|&(sx, sy)| self.pixel(x + sx, y + sy) == image.pixel(sx, sy))
            && (0..image.height).all(|iy| {
                let at = ((y + iy) as usize * self.width as usize + x as usize) * 4;
                let from = iy as usize * row;
                self.pixels[at..at + row] == image.pixels[from..from + row]
            })
    }
}

/// 画素で探すときに試す位置の数の上限 (大きな画像で時間がかかりすぎないように)。
const SEARCH_LIMIT: usize = 1 << 16;
/// 画素で探すとき、先に見る不透明な画素の数。
const SAMPLE_COUNT: usize = 32;

/// アトラスの中で今の編集の画像を置いた場所を探す。
struct Finder<'a> {
    file: &'a ModuleFile,
    module: &'a Module,
    images: &'a Textures,
    atlases: BTreeMap<usize, Option<Rgba>>,
    textures: BTreeMap<usize, Option<Rgba>>,
}

/// ファイルのメッシュ 1 つの戻し先 (今の編集のテクスチャの番号・置いた場所・画素が同じか)。
type Found = Option<(usize, Placement, bool)>;

impl<'a> Finder<'a> {
    fn new(file: &'a ModuleFile, module: &'a Module, images: &'a Textures) -> Self {
        Self {
            file,
            module,
            images,
            atlases: BTreeMap::new(),
            textures: BTreeMap::new(),
        }
    }

    /// アトラス `atlases` を使うファイルのメッシュごとの戻し先 (並びはファイルのメッシュと同じ)。
    ///
    /// 1. 今の編集に同じ ID のメッシュがあれば、そのテクスチャに戻す。場所は UV の組から推し量り (多数決)、
    ///    画素で確かめる。確かめられず、多数決も過半数に届かなければ、画素で探す。
    /// 2. 今の編集に無いメッシュ (除いたもの) は、見つけた場所の中にあればそのテクスチャ、無ければ
    ///    まだ見つけていない今の編集のテクスチャを画素で探す。
    fn resolve(&mut self, atlases: &BTreeSet<usize>, candidates: &[usize]) -> Vec<Found> {
        let old = &self.file.module;
        let mut found: Vec<Found> = vec![None; old.meshes.len()];
        let mut groups: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
        let mut orphans = Vec::new();
        for (k, mesh) in old.meshes.iter().enumerate() {
            let a = mesh.texture as usize;
            if !atlases.contains(&a) {
                continue;
            }
            match self.module.meshes.iter().find(|m| m.id == mesh.id) {
                Some(current) if candidates.contains(&(current.texture as usize)) => {
                    groups
                        .entry((a, current.texture as usize))
                        .or_default()
                        .push(k);
                }
                Some(_) => {}
                None => orphans.push(k),
            }
        }
        let mut placed: Vec<(usize, usize, Placement, bool)> = Vec::new();
        for ((a, t), members) in groups {
            if let Some((place, same)) = self.locate(a, t, &members) {
                for &k in &members {
                    found[k] = Some((t, place, same));
                }
                placed.push((a, t, place, same));
            }
        }
        for k in orphans {
            let a = old.meshes[k].texture as usize;
            let Some(bounds) = self.bounds(a, &[k]) else {
                continue;
            };
            let inside = placed.iter().find(|(pa, _, place, _)| {
                *pa == a
                    && f64::from(place.x) <= bounds.0 + EPSILON
                    && f64::from(place.y) <= bounds.1 + EPSILON
                    && f64::from(place.x + place.width) >= bounds.2 - EPSILON
                    && f64::from(place.y + place.height) >= bounds.3 - EPSILON
            });
            if let Some(&(_, t, place, same)) = inside {
                found[k] = Some((t, place, same));
                continue;
            }
            for &t in candidates {
                if placed.iter().any(|(_, pt, _, _)| *pt == t) {
                    continue;
                }
                if let Some(place) = self.search(a, t, bounds) {
                    found[k] = Some((t, place, true));
                    placed.push((a, t, place, true));
                    break;
                }
            }
        }
        found
    }

    /// ファイルのメッシュ `members` (今の編集ではテクスチャ `t` を使う) がアトラス `a` で使う場所と、画素が同じか。
    fn locate(&mut self, a: usize, t: usize, members: &[usize]) -> Option<(Placement, bool)> {
        let old = &self.file.module;
        let (w, h) = self.texture_size(t);
        let atlas = &old.textures[a];
        let (aw, ah) = (atlas.width, atlas.height);
        // UV の組ごとに、置いた左上の画素を推し量る。
        let mut votes: BTreeMap<(i64, i64), usize> = BTreeMap::new();
        let mut total = 0;
        for &k in members {
            let mesh = &old.meshes[k];
            let Some(current) = self.module.meshes.iter().find(|m| m.id == mesh.id) else {
                continue;
            };
            if current.uvs.len() != mesh.uvs.len() {
                continue;
            }
            for (f, c) in mesh.uvs.iter().zip(&current.uvs) {
                let offset = |f: f32, c: f32, size: u32, total: u32| {
                    (f64::from(f) * f64::from(total) - f64::from(c) * f64::from(size)).round()
                        as i64
                };
                *votes
                    .entry((offset(f.x, c.x, w, aw), offset(f.y, c.y, h, ah)))
                    .or_default() += 1;
                total += 1;
            }
        }
        // 票の多い順 (同数なら左上から)。
        let best = votes
            .iter()
            .max_by(|x, y| x.1.cmp(y.1).then(y.0.cmp(x.0)))
            .map(|(&xy, &n)| (xy, n));
        if let Some(((x, y), n)) = best
            && let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y))
            && u64::from(x) + u64::from(w) <= u64::from(aw)
            && u64::from(y) + u64::from(h) <= u64::from(ah)
        {
            let place = placement(x, y, (w, h), (aw, ah));
            let same = self.pixels_match(a, t, x, y);
            if same || n * 2 > total {
                return Some((place, same));
            }
        }
        let bounds = self.bounds(a, members)?;
        self.search(a, t, bounds).map(|place| (place, true))
    }

    /// アトラス `a` の上で、ファイルのメッシュ `members` の UV が占める範囲 (画素。左・上・右・下)。
    fn bounds(&self, a: usize, members: &[usize]) -> Option<(f64, f64, f64, f64)> {
        let old = &self.file.module;
        let atlas = &old.textures[a];
        let uvs: Vec<Vec2> = members
            .iter()
            .flat_map(|&k| old.meshes[k].uvs.iter().copied())
            .collect();
        if uvs.is_empty() || uvs.iter().any(|uv| !uv.is_finite()) {
            return None;
        }
        let mut bounds = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for uv in uvs {
            let (x, y) = (
                f64::from(uv.x) * f64::from(atlas.width),
                f64::from(uv.y) * f64::from(atlas.height),
            );
            bounds = (
                bounds.0.min(x),
                bounds.1.min(y),
                bounds.2.max(x),
                bounds.3.max(y),
            );
        }
        Some(bounds)
    }

    /// アトラス `a` の中で、範囲 `bounds` を覆う位置に今の編集の画像 `t` と同じ画素が並ぶ場所を探す。
    fn search(&mut self, a: usize, t: usize, bounds: (f64, f64, f64, f64)) -> Option<Placement> {
        self.decode(a, t);
        let atlas = self.atlases.get(&a)?.as_ref()?;
        let image = self.textures.get(&t)?.as_ref()?;
        if image.width > atlas.width || image.height > atlas.height {
            return None;
        }
        let range = |min: f64, max: f64, size: u32, total: u32| {
            let from = (max - f64::from(size) - EPSILON).ceil().max(0.0);
            let to = (min + EPSILON).floor().min(f64::from(total - size));
            (from <= to).then_some((from as u32, to as u32))
        };
        let (x0, x1) = range(bounds.0, bounds.2, image.width, atlas.width)?;
        let (y0, y1) = range(bounds.1, bounds.3, image.height, atlas.height)?;
        let tries = (x1 - x0 + 1) as usize * (y1 - y0 + 1) as usize;
        if tries > SEARCH_LIMIT {
            return None;
        }
        // 透明な画素はアトラスの余白にもあるので、不透明な画素を先に見る。すべて透明な画像は探さない。
        let opaque: Vec<(u32, u32)> = (0..image.height)
            .flat_map(|y| (0..image.width).map(move |x| (x, y)))
            .filter(|&(x, y)| image.pixel(x, y)[3] != 0)
            .collect();
        if opaque.is_empty() {
            return None;
        }
        let step = opaque.len().div_ceil(SAMPLE_COUNT);
        let samples: Vec<(u32, u32)> = opaque.into_iter().step_by(step).collect();
        let (w, h) = (image.width, image.height);
        (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
            .find(|&(x, y)| atlas.contains_at(image, x, y, &samples))
            .map(|(x, y)| placement(x, y, (w, h), (atlas.width, atlas.height)))
    }

    /// アトラス `a` の `(x, y)` に今の編集の画像 `t` と同じ画素が並ぶか。
    fn pixels_match(&mut self, a: usize, t: usize, x: u32, y: u32) -> bool {
        self.decode(a, t);
        match (self.atlases.get(&a), self.textures.get(&t)) {
            (Some(Some(atlas)), Some(Some(image))) => atlas.contains_at(image, x, y, &[]),
            _ => false,
        }
    }

    /// 今の編集の画像 `t` の大きさ (読めなければテクスチャの参照の大きさ)。
    fn texture_size(&mut self, t: usize) -> (u32, u32) {
        self.decode_texture(t);
        match self.textures.get(&t) {
            Some(Some(image)) => (image.width, image.height),
            _ => {
                let texture = &self.module.textures[t];
                (texture.width, texture.height)
            }
        }
    }

    fn decode(&mut self, a: usize, t: usize) {
        let file = self.file;
        self.atlases.entry(a).or_insert_with(|| {
            file.textures
                .get(a)
                .and_then(|data| Rgba::decode(&data.bytes))
        });
        self.decode_texture(t);
    }

    fn decode_texture(&mut self, t: usize) {
        let (module, images) = (self.module, self.images);
        self.textures.entry(t).or_insert_with(|| {
            images
                .get(&module.textures[t].name)
                .and_then(|png| Rgba::decode(png))
        });
    }
}

/// UV から求めた画素の位置の丸めの許し (画素)。
const EPSILON: f64 = 1e-3;

fn placement(x: u32, y: u32, (width, height): (u32, u32), (aw, ah): (u32, u32)) -> Placement {
    Placement {
        x,
        y,
        width,
        height,
        atlas_width: aw,
        atlas_height: ah,
    }
}
