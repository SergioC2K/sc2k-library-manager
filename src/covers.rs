//! Portadas: cache de texturas con carga perezosa y sustituto provisional.
//!
//! La decodificación ocurre en un hilo de trabajo, nunca en el hilo de dibujo.
//! Si la pista no tiene portada (o el archivo falla), el mismo hilo rasteriza un
//! sustituto estable a partir del color de la pista y la inicial del título.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use egui::{Color32, ColorImage, Context, Pos2, Rect, TextureHandle, TextureOptions, Ui, Vec2};

use crate::artwork;
use crate::harmony::{self, text_color_for};
use crate::model::{CoverSource, Track, TrackId};

const TEXTURE_SIZE: usize = 128;
/// Lado máximo de las portadas reales; se reduce al cargarlas.
const MAX_COVER_SIDE: u32 = 256;
/// Máximo de peticiones en vuelo para no inundar el hilo de portadas.
const MAX_PENDING: usize = 48;
/// Máximo de texturas residentes; se descartan las que no se vieron el frame anterior.
const MAX_TEXTURES: usize = 256;

struct CoverJob {
    id: TrackId,
    source: Option<CoverSource>,
    base: Color32,
    seed: u32,
}

struct CoverReady {
    id: TrackId,
    image: ColorImage,
    /// `false` si es el sustituto: en ese caso se dibuja la inicial encima.
    real: bool,
}

struct CoverTexture {
    handle: TextureHandle,
    real: bool,
}

/// Cache de texturas de portada con carga perezosa en segundo plano.
pub struct CoverCache {
    textures: HashMap<TrackId, CoverTexture>,
    pending: HashSet<TrackId>,
    seen: HashSet<TrackId>,
    previous_seen: HashSet<TrackId>,
    jobs: Sender<CoverJob>,
    ready: Receiver<CoverReady>,
}

impl CoverCache {
    pub fn new(ctx: &Context) -> Self {
        let (jobs, job_rx) = mpsc::channel::<CoverJob>();
        let (ready_tx, ready) = mpsc::channel::<CoverReady>();
        let worker_ctx = ctx.clone();
        thread::spawn(move || {
            while let Ok(job) = job_rx.recv() {
                let decoded = job.source.as_ref().and_then(decode_cover);
                let real = decoded.is_some();
                let image = decoded.unwrap_or_else(|| render_placeholder(job.base, job.seed));
                let sent = ready_tx
                    .send(CoverReady {
                        id: job.id,
                        image,
                        real,
                    })
                    .is_ok();
                if !sent {
                    break;
                }
                worker_ctx.request_repaint();
            }
        });

        Self {
            textures: HashMap::new(),
            pending: HashSet::new(),
            seen: HashSet::new(),
            previous_seen: HashSet::new(),
            jobs,
            ready,
        }
    }

    /// Sube a la GPU las texturas listas y descarta las que ya no se ven.
    /// Se llama una vez por frame, antes de dibujar.
    pub fn poll(&mut self, ctx: &Context) {
        while let Ok(ready) = self.ready.try_recv() {
            self.pending.remove(&ready.id);
            let texture = ctx.load_texture(
                format!("portada-{}", ready.id),
                ready.image,
                TextureOptions::LINEAR,
            );
            self.textures.insert(
                ready.id,
                CoverTexture {
                    handle: texture,
                    real: ready.real,
                },
            );
        }

        if self.textures.len() > MAX_TEXTURES {
            let keep = &self.previous_seen;
            self.textures.retain(|id, _| keep.contains(id));
        }
        std::mem::swap(&mut self.seen, &mut self.previous_seen);
        self.seen.clear();
    }

    /// Vacía la cache (por ejemplo al cargar otra librería).
    pub fn clear(&mut self) {
        self.textures.clear();
        self.pending.clear();
        self.seen.clear();
        self.previous_seen.clear();
    }

    /// Dibuja la portada en `rect` y solicita sus píxeles si aún no existen.
    pub fn draw(&mut self, ui: &Ui, track: &Track, rect: Rect) {
        let base = base_color(track);
        self.seen.insert(track.id);

        if !self.textures.contains_key(&track.id)
            && !self.pending.contains(&track.id)
            && self.pending.len() < MAX_PENDING
        {
            let seed = (track.id as u32).wrapping_mul(2_654_435_761);
            if self
                .jobs
                .send(CoverJob {
                    id: track.id,
                    source: track.cover.clone(),
                    base,
                    seed,
                })
                .is_ok()
            {
                self.pending.insert(track.id);
            }
        }

        let painter = ui.painter();
        match self.textures.get(&track.id) {
            Some(texture) if texture.real => {
                painter.image(
                    texture.handle.id(),
                    rect,
                    cover_uv(texture.handle.size_vec2(), rect.size()),
                    Color32::WHITE,
                );
            }
            Some(texture) => {
                painter.image(texture.handle.id(), rect, full_uv(), Color32::WHITE);
                paint_initial(painter, track, rect, base);
            }
            None => {
                painter.rect_filled(rect, 3.0, base);
                paint_initial(painter, track, rect, base);
            }
        }
    }
}

fn paint_initial(painter: &egui::Painter, track: &Track, rect: Rect, base: Color32) {
    let font_size = (rect.height() * 0.34).clamp(12.0, 40.0);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        initial(track),
        egui::FontId::proportional(font_size),
        text_color_for(base).gamma_multiply(0.92),
    );
}

/// Decodifica la portada de una pista, fuera del hilo de dibujo.
fn decode_cover(source: &CoverSource) -> Option<ColorImage> {
    match source {
        CoverSource::Embedded(path) => {
            let bytes = artwork::embedded_image(path)?;
            decode_image_bytes(&bytes)
        }
        CoverSource::RawRgba(path) => {
            let bytes = std::fs::read(path).ok()?;
            let raw = artwork::raw_rgba(&bytes)?;
            let image = image::RgbaImage::from_raw(raw.width, raw.height, raw.pixels)?;
            Some(rgba_to_color_image(image))
        }
    }
}

fn decode_image_bytes(bytes: &[u8]) -> Option<ColorImage> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8_192);
    limits.max_image_height = Some(8_192);
    limits.max_alloc = Some(256 * 1024 * 1024);

    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    reader.limits(limits);
    let decoded = reader.decode().ok()?;
    let thumbnail = decoded.thumbnail(MAX_COVER_SIDE, MAX_COVER_SIDE);
    Some(rgba_to_color_image(thumbnail.to_rgba8()))
}

fn rgba_to_color_image(image: image::RgbaImage) -> ColorImage {
    // `thumbnail` solo reduce; las portadas ya pequeñas (125×125 en Traktor) se
    // quedan como están y no se deforman.
    let thumbnail = if image.width() > MAX_COVER_SIDE || image.height() > MAX_COVER_SIDE {
        image::DynamicImage::ImageRgba8(image)
            .thumbnail(MAX_COVER_SIDE, MAX_COVER_SIDE)
            .to_rgba8()
    } else {
        image
    };
    let (width, height) = thumbnail.dimensions();
    let pixels = thumbnail
        .pixels()
        .map(|pixel| Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]))
        .collect();
    ColorImage {
        size: [width as usize, height as usize],
        pixels,
    }
}

fn full_uv() -> Rect {
    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0))
}

/// UV para rellenar `target` recortando la imagen (sin deformarla).
fn cover_uv(image: Vec2, target: Vec2) -> Rect {
    if image.x <= 0.0 || image.y <= 0.0 || target.x <= 0.0 || target.y <= 0.0 {
        return full_uv();
    }
    let image_aspect = image.x / image.y;
    let target_aspect = target.x / target.y;

    if image_aspect > target_aspect {
        let width = target_aspect / image_aspect;
        let x0 = (1.0 - width) / 2.0;
        Rect::from_min_max(Pos2::new(x0, 0.0), Pos2::new(1.0 - x0, 1.0))
    } else {
        let height = image_aspect / target_aspect;
        let y0 = (1.0 - height) / 2.0;
        Rect::from_min_max(Pos2::new(0.0, y0), Pos2::new(1.0, 1.0 - y0))
    }
}

/// Color base de la portada provisional: el de la clave o uno estable por pista.
pub fn base_color(track: &Track) -> Color32 {
    match track.key {
        Some(key) => key.color(),
        None => {
            let hue = (track.id as u32).wrapping_mul(37) % 360;
            harmony::color_from_hsv(hue as f32, 0.32, 0.62)
        }
    }
}

fn initial(track: &Track) -> String {
    track
        .title
        .chars()
        .find(|character| character.is_alphanumeric())
        .map(|character| character.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
}

fn render_placeholder(base: Color32, seed: u32) -> ColorImage {
    let top = scale(base, 1.25);
    let bottom = scale(base, 0.55);
    let phase = (seed % 360) as f32 * std::f32::consts::PI / 180.0;
    let last = (TEXTURE_SIZE - 1) as f32;

    let mut pixels = Vec::with_capacity(TEXTURE_SIZE * TEXTURE_SIZE);
    for y in 0..TEXTURE_SIZE {
        let vertical = y as f32 / last;
        let row = mix(top, bottom, vertical);
        for x in 0..TEXTURE_SIZE {
            let diagonal = (x as f32 / last + vertical) * 0.5;
            let glow = ((diagonal * 5.0 + phase).sin() * 0.5 + 0.5) * 0.12;
            pixels.push(mix(row, Color32::WHITE, glow));
        }
    }

    ColorImage {
        size: [TEXTURE_SIZE, TEXTURE_SIZE],
        pixels,
    }
}

fn scale(color: Color32, factor: f32) -> Color32 {
    let channel = |value: u8| (f32::from(value) * factor).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(channel(color.r()), channel(color.g()), channel(color.b()))
}

fn mix(from: Color32, to: Color32, amount: f32) -> Color32 {
    let amount = amount.clamp(0.0, 1.0);
    let channel =
        |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * amount).round() as u8;
    Color32::from_rgb(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TrackColor;
    use std::time::Duration;

    fn track(title: &str) -> Track {
        Track {
            id: 7,
            title: title.to_string(),
            artist: "Artista".to_string(),
            album: "Album".to_string(),
            bpm: 120.0,
            key: None,
            duration: Duration::from_secs(200),
            color: TrackColor::None,
            cue_points: 0,
            cover: None,
        }
    }

    #[test]
    fn initial_uses_first_alphanumeric_character() {
        assert_eq!(initial(&track("  neon district")), "N");
        assert_eq!(initial(&track("3 a.m.")), "3");
        assert_eq!(initial(&track("...")), "?");
    }

    #[test]
    fn placeholder_has_the_expected_size_and_is_opaque() {
        let image = render_placeholder(Color32::from_rgb(120, 60, 200), 42);
        assert_eq!(image.size, [TEXTURE_SIZE, TEXTURE_SIZE]);
        assert_eq!(image.pixels.len(), TEXTURE_SIZE * TEXTURE_SIZE);
        assert!(image.pixels.iter().all(|pixel| pixel.a() == 255));
    }

    #[test]
    fn base_color_is_stable_for_tracks_without_key() {
        let first = track("Uno");
        let mut second = first.clone();
        second.id = 2_000;
        assert_eq!(base_color(&first), base_color(&first));
        assert_ne!(base_color(&first), base_color(&second));
    }

    #[test]
    fn square_images_use_the_whole_texture() {
        let uv = cover_uv(Vec2::splat(128.0), Vec2::splat(40.0));
        assert_eq!(uv.min, Pos2::new(0.0, 0.0));
        assert_eq!(uv.max, Pos2::new(1.0, 1.0));
    }

    #[test]
    fn wide_images_are_cropped_horizontally() {
        let uv = cover_uv(Vec2::new(200.0, 100.0), Vec2::splat(100.0));
        assert!((uv.min.x - 0.25).abs() < 1e-6);
        assert!((uv.max.x - 0.75).abs() < 1e-6);
        assert_eq!((uv.min.y, uv.max.y), (0.0, 1.0));
    }

    #[test]
    fn tall_images_are_cropped_vertically() {
        let uv = cover_uv(Vec2::new(100.0, 200.0), Vec2::splat(100.0));
        assert!((uv.min.y - 0.25).abs() < 1e-6);
        assert!((uv.max.y - 0.75).abs() < 1e-6);
        assert_eq!((uv.min.x, uv.max.x), (0.0, 1.0));
    }

    #[test]
    fn jpeg_and_png_bytes_decode_to_a_bounded_texture() {
        use image::ImageEncoder;

        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        let pixels: Vec<u8> = (0..600 * 400)
            .flat_map(|_| [200u8, 150, 100, 255])
            .collect();
        encoder
            .write_image(&pixels, 600, 400, image::ExtendedColorType::Rgba8)
            .expect("PNG de prueba");

        let image = decode_image_bytes(&png).expect("PNG válido");
        assert_eq!(image.size, [256, 171]);
        assert!(image.pixels.iter().all(|pixel| pixel.a() == 255));

        assert!(decode_image_bytes(b"no es una imagen").is_none());
    }

    /// Comprobación manual con las librerías reales del equipo.
    #[test]
    #[ignore = "lee las librerías locales del equipo; ejecutar a mano"]
    fn real_covers_decode_end_to_end() {
        use crate::model::CoverSource;
        use std::path::PathBuf;

        let Some(home) = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
        else {
            return;
        };

        let mut total = 0;

        if let Some(appdata) = std::env::var_os("APPDATA") {
            let xml = PathBuf::from(appdata)
                .join("Pioneer")
                .join("rekordbox")
                .join("rekordbox.xml");
            if xml.is_file() {
                let library =
                    crate::readers::rekordbox_xml::read(&xml, &mut |_| {}).expect("rekordbox");
                let mut tried = 0;
                let mut decoded = 0;
                for source in library
                    .tracks
                    .iter()
                    .filter_map(|track| track.cover.as_ref())
                    .take(600)
                {
                    tried += 1;
                    if let Some(image) = decode_cover(source) {
                        assert!(image.size[0] > 0 && image.size[1] > 0);
                        assert!(!image.pixels.is_empty());
                        decoded += 1;
                    }
                }
                println!("Rekordbox: {decoded}/{tried} portadas decodificadas");
                assert!(
                    decoded > 0,
                    "ninguna portada de Rekordbox se pudo decodificar"
                );
                total += decoded;
            }
        }

        let native = home.join("Documents").join("Native Instruments");
        if let Ok(entries) = std::fs::read_dir(&native) {
            let mut collections: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path().join("collection.nml"))
                .filter(|path| path.is_file())
                .collect();
            collections.sort();
            if let Some(path) = collections.last() {
                let library = crate::readers::traktor::read(path, &mut |_| {}).expect("traktor");
                let covers: Vec<&CoverSource> = library
                    .tracks
                    .iter()
                    .filter_map(|track| track.cover.as_ref())
                    .take(30)
                    .collect();
                let decoded = covers
                    .iter()
                    .filter(|source| decode_cover(source).is_some())
                    .count();
                println!("Traktor: {decoded}/{} portadas decodificadas", covers.len());
                assert!(
                    decoded > 0,
                    "ninguna portada de Traktor se pudo decodificar"
                );
                total += decoded;
            }
        }

        assert!(total > 0);
    }
}
