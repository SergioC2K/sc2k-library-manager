//! Armonia musical en notacion Camelot y colores de clave.
//!
//! Toda la logica de este modulo es pura (sin `egui::Ui` ni estado global),
//! de modo que se puede probar con tests unitarios y reutilizar desde el
//! agente de armonia cuando exista `crates/harmony`.

use egui::Color32;

/// Modo de la tonalidad: menor (A) o mayor (B) en notacion Camelot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyMode {
    Minor,
    Major,
}

impl KeyMode {
    /// Letra Camelot asociada.
    pub const fn letter(self) -> &'static str {
        match self {
            Self::Minor => "A",
            Self::Major => "B",
        }
    }

    /// Nombre legible en espanol.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Minor => "menor",
            Self::Major => "mayor",
        }
    }
}

/// Clave en notacion Camelot: numero de 1 a 12 y modo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CamelotKey {
    pub number: u8,
    pub mode: KeyMode,
}

impl CamelotKey {
    /// Crea una clave valida (1A-12B) o `None` si el numero esta fuera de rango.
    pub fn new(number: u8, mode: KeyMode) -> Option<Self> {
        (1..=12).contains(&number).then_some(Self { number, mode })
    }

    /// Interpreta textos como `8A`, `12b` o ` 3 B `.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let mut chars = text.chars();
        let letter = chars.next_back()?.to_ascii_uppercase();
        let digits = chars.as_str().trim();
        let number: u8 = digits.parse().ok()?;
        let mode = match letter {
            'A' => KeyMode::Minor,
            'B' => KeyMode::Major,
            _ => return None,
        };
        Self::new(number, mode)
    }

    /// Etiqueta Camelot, por ejemplo `8A`.
    pub fn label(self) -> String {
        format!("{}{}", self.number, self.mode.letter())
    }

    /// Dos claves son compatibles si son relativa mayor/menor (misma posicion)
    /// o si estan a +-1 con la misma letra, incluyendo la vuelta de 12 a 1.
    pub fn is_compatible(self, other: Self) -> bool {
        if self.number == other.number {
            return true;
        }
        if self.mode != other.mode {
            return false;
        }
        let distance = (i16::from(self.number) - i16::from(other.number)).rem_euclid(12);
        distance == 1 || distance == 11
    }

    /// Color derivado del numero Camelot (matiz = posicion).
    /// Menor (A) mas saturado; mayor (B) mas suave.
    pub fn color(self) -> Color32 {
        let hue = (f32::from(self.number) - 1.0) * 30.0;
        let (saturation, value) = match self.mode {
            KeyMode::Minor => (0.48, 0.99),
            KeyMode::Major => (0.26, 1.0),
        };
        color_from_hsv(hue, saturation, value)
    }
}

/// Color a partir de matiz (grados), saturacion y valor, todos en 0..=1.
pub fn color_from_hsv(hue_degrees: f32, saturation: f32, value: f32) -> Color32 {
    let (red, green, blue) = hsv_to_rgb(hue_degrees, saturation, value);
    Color32::from_rgb(red, green, blue)
}

fn hsv_to_rgb(hue_degrees: f32, saturation: f32, value: f32) -> (u8, u8, u8) {
    let hue = hue_degrees.rem_euclid(360.0) / 60.0;
    let saturation = saturation.clamp(0.0, 1.0);
    let value = value.clamp(0.0, 1.0);

    let chroma = value * saturation;
    let second = chroma * (1.0 - (hue % 2.0 - 1.0).abs());
    let base = value - chroma;

    let (red, green, blue) = match hue as u32 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };

    let to_byte = |channel: f32| ((channel + base) * 255.0).round().clamp(0.0, 255.0) as u8;
    (to_byte(red), to_byte(green), to_byte(blue))
}

/// Color de texto legible sobre `background` (negro o blanco).
pub fn text_color_for(background: Color32) -> Color32 {
    let luminance = 0.299 * f32::from(background.r())
        + 0.587 * f32::from(background.g())
        + 0.114 * f32::from(background.b());
    if luminance > 140.0 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(number: u8, mode: KeyMode) -> CamelotKey {
        CamelotKey::new(number, mode).expect("clave valida")
    }

    #[test]
    fn parse_and_label_roundtrip() {
        let parsed = CamelotKey::parse(" 8a ").expect("clave 8A");
        assert_eq!(parsed, key(8, KeyMode::Minor));
        assert_eq!(parsed.label(), "8A");

        let parsed = CamelotKey::parse("12B").expect("clave 12B");
        assert_eq!(parsed, key(12, KeyMode::Major));
        assert_eq!(parsed.label(), "12B");
    }

    #[test]
    fn parse_rejects_invalid_keys() {
        assert!(CamelotKey::parse("").is_none());
        assert!(CamelotKey::parse("13A").is_none());
        assert!(CamelotKey::parse("0B").is_none());
        assert!(CamelotKey::parse("8C").is_none());
        assert!(CamelotKey::parse("A8").is_none());
        assert!(CamelotKey::parse("8").is_none());
    }

    #[test]
    fn relative_major_minor_is_compatible() {
        assert!(key(8, KeyMode::Minor).is_compatible(key(8, KeyMode::Major)));
        assert!(key(8, KeyMode::Major).is_compatible(key(8, KeyMode::Minor)));
    }

    #[test]
    fn neighbours_with_same_letter_are_compatible() {
        assert!(key(4, KeyMode::Minor).is_compatible(key(5, KeyMode::Minor)));
        assert!(key(5, KeyMode::Minor).is_compatible(key(4, KeyMode::Minor)));
        assert!(key(7, KeyMode::Major).is_compatible(key(6, KeyMode::Major)));
    }

    #[test]
    fn compatibility_wraps_from_12_to_1() {
        assert!(key(12, KeyMode::Minor).is_compatible(key(1, KeyMode::Minor)));
        assert!(key(1, KeyMode::Minor).is_compatible(key(12, KeyMode::Minor)));
        assert!(key(12, KeyMode::Major).is_compatible(key(1, KeyMode::Major)));
    }

    #[test]
    fn distant_or_mixed_letter_neighbours_are_not_compatible() {
        assert!(!key(1, KeyMode::Minor).is_compatible(key(3, KeyMode::Minor)));
        assert!(!key(6, KeyMode::Minor).is_compatible(key(7, KeyMode::Major)));
        assert!(!key(2, KeyMode::Major).is_compatible(key(9, KeyMode::Major)));
        assert!(!key(3, KeyMode::Minor).is_compatible(key(11, KeyMode::Minor)));
    }

    #[test]
    fn compatibility_is_symmetric() {
        for number in 1..=12u8 {
            for mode in [KeyMode::Minor, KeyMode::Major] {
                for other_number in 1..=12u8 {
                    for other_mode in [KeyMode::Minor, KeyMode::Major] {
                        let a = key(number, mode);
                        let b = key(other_number, other_mode);
                        assert_eq!(a.is_compatible(b), b.is_compatible(a));
                    }
                }
            }
        }
    }

    #[test]
    fn key_colors_are_light_enough_for_black_text() {
        // El chip usa siempre texto negro; todos los colores deben tener
        // luminancia suficiente para mantener el contraste.
        for number in 1..=12u8 {
            for mode in [KeyMode::Minor, KeyMode::Major] {
                let color = key(number, mode).color();
                let luminance = 0.299 * f32::from(color.r())
                    + 0.587 * f32::from(color.g())
                    + 0.114 * f32::from(color.b());
                assert!(
                    luminance > 140.0,
                    "clave {number}{} demasiado oscura",
                    mode.letter()
                );
                assert_eq!(text_color_for(color), Color32::BLACK);
            }
        }
    }

    #[test]
    fn hues_follow_camelot_position() {
        let first = key(1, KeyMode::Minor).color();
        let seventh = key(7, KeyMode::Minor).color();
        assert_ne!(first, seventh);
        assert_eq!(first, color_from_hsv(0.0, 0.48, 0.99));
    }

    #[test]
    fn text_color_for_extremes() {
        assert_eq!(text_color_for(Color32::WHITE), Color32::BLACK);
        assert_eq!(text_color_for(Color32::BLACK), Color32::WHITE);
    }

    #[test]
    fn hsv_converts_primary_colors() {
        assert_eq!(color_from_hsv(0.0, 1.0, 1.0), Color32::from_rgb(255, 0, 0));
        assert_eq!(
            color_from_hsv(120.0, 1.0, 1.0),
            Color32::from_rgb(0, 255, 0)
        );
        assert_eq!(
            color_from_hsv(240.0, 1.0, 1.0),
            Color32::from_rgb(0, 0, 255)
        );
        assert_eq!(color_from_hsv(360.0, 0.0, 1.0), Color32::WHITE);
    }
}
