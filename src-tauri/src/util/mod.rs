//! Hilfsfunktionen ohne Netzwerk und ohne Tauri, deshalb gut zu testen.

mod ics;
mod net;
mod paths;
mod security;
#[cfg(test)]
mod tests;
mod text;
mod time;

pub use ics::build_ics;
pub use net::{filename_from_url, is_private_host, normalize_url, parse_content_disposition, url_allowed};
pub use paths::{
    fit_components, is_forbidden_dest, render_dir_template, sanitize_component, sanitize_filename, unique_path,
    unique_rel, validate_export_path,
};
pub use security::{is_executable_ext, redact, redact_url};
pub use text::{collapse_ws, ext_of, fmt_bytes, html_unescape, lock};
pub use time::{iso_utc, now_ts};
