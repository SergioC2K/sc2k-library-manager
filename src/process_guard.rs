//! Comprobación de que los programas de DJ están cerrados.
//!
//! Antes de leer una librería se comprueba que Rekordbox o Traktor no estén en
//! marcha: así evitamos bloqueos de archivo y lecturas a medio escribir. La
//! detección en sí es de solo lectura y nunca modifica nada del usuario.

use std::process::Command;

/// Nombres de proceso (sin distinguir mayúsculas) de Rekordbox.
pub const REKORDBOX_PROCESSES: &[&str] = &["rekordbox.exe", "rekordbox"];
/// Nombres de proceso (sin distinguir mayúsculas) de Traktor.
pub const TRAKTOR_PROCESSES: &[&str] = &["traktor.exe", "traktor"];

/// `true` si alguno de los procesos indicados está en ejecución.
pub fn any_running(names: &[&str]) -> bool {
    any_running_in(&task_list(), names)
}

/// Comprueba una salida de `tasklist` sin depender del sistema operativo.
/// Se separa de `any_running` para poder probarla con datos fijos.
pub fn any_running_in(tasklist_output: &str, names: &[&str]) -> bool {
    let wanted: Vec<String> = names
        .iter()
        .map(|name| name.trim().to_ascii_lowercase())
        .filter(|name| !name.is_empty())
        .collect();
    image_names(tasklist_output)
        .iter()
        .any(|running| wanted.iter().any(|name| name == running))
}

/// Nombres de imagen de la primera columna de `tasklist /FO CSV /NH`.
fn image_names(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| line.split(',').next())
        .map(str::trim)
        .map(|field| field.trim_matches('"'))
        .filter(|field| !field.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// Salida de `tasklist`; vacía si no se puede consultar.
fn task_list() -> String {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        // CREATE_NO_WINDOW: no abrir una consola al consultar los procesos.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
            .unwrap_or_default()
    }

    #[cfg(not(windows))]
    {
        // TODO(macOS/Linux): usar `pgrep -x` en lugar de `tasklist`.
        // Mientras tanto se considera que no hay programas abiertos.
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_tasklist_rows_are_parsed() {
        let output = "\"Rekordbox.exe\",\"1234\",\"Console\",\"1\",\"120.000 K\"\r\n\
                      \"explorer.exe\",\"5678\",\"Console\",\"1\",\"80.000 K\"\r\n";
        assert!(any_running_in(output, REKORDBOX_PROCESSES));
        assert!(!any_running_in(output, TRAKTOR_PROCESSES));
    }

    #[test]
    fn process_names_are_case_insensitive() {
        let output = "\"TRAKTOR.EXE\",\"42\",\"Console\",\"1\",\"1.000 K\"\n";
        assert!(any_running_in(output, TRAKTOR_PROCESSES));
    }

    #[test]
    fn similar_names_do_not_trigger_false_positives() {
        let output = "\"rekordboxhelper.exe\",\"1\",\"Console\",\"1\",\"1 K\"\n\
                      \"traktor_installer.exe\",\"2\",\"Console\",\"1\",\"1 K\"\n";
        assert!(!any_running_in(output, REKORDBOX_PROCESSES));
        assert!(!any_running_in(output, TRAKTOR_PROCESSES));
    }

    #[test]
    fn empty_or_unreadable_output_means_not_running() {
        assert!(!any_running_in("", REKORDBOX_PROCESSES));
        assert!(!any_running_in(
            "INFO: No tasks are running",
            TRAKTOR_PROCESSES
        ));
        assert!(!any_running_in("\"rekordbox.exe\"", &[]));
    }
}
