use crate::i18n::lang::Lang;

pub fn queued_note(lang: Lang) -> String {
    match lang {
        Lang::En => "Eligible, but admissions are full for this 30-day window (rate cap). You are queued by qualification date.".to_string(),
        Lang::Es => "Elegible, pero las admisiones están completas en esta ventana de 30 días (límite de ritmo). Estás en cola por fecha de calificación.".to_string(),
    }
}
