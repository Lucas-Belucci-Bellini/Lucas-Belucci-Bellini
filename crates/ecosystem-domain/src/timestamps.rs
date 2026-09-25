//! Datas como o Python as lê: `datetime.fromisoformat(value.replace("Z", "+00:00"))`.
//!
//! O `fromisoformat` do CPython é bem mais permissivo que o RFC 3339 do
//! `chrono`, e de formas que ninguém adivinharia: `10.5` vira 10:00:00,5;
//! `+00:60` é um fuso de uma hora; qualquer caractere serve de separador entre
//! data e hora, e um caractere solto entre a hora e o fuso é ignorado. Em vez
//! de aproximar, este módulo **porta o código C** de
//! `Modules/_datetimemodule.c` do CPython 3.12 (a versão do CI), função por
//! função e sobre os mesmos bytes UTF-8 — os nomes abaixo são os de lá. Cada
//! peculiaridade é um caso do fixture de paridade.
//!
//! O código atual do ramo 3.13 do CPython (posterior ao 3.13.12, que se
//! comporta como o 3.12) muda duas bordas: `.` sem dígitos passa a ser
//! recusado, e fuso só com microssegundos deixa de virar UTC. Os dois casos
//! estão no fixture: um Python do CI com essa mudança reprova
//! `tests/test_parity_domain.py` em vez de mudar a referência em silêncio.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc, Weekday};

/// O que `datetime.fromisoformat()` devolve para um texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyIsoDatetime {
    /// Data e hora com fuso, convertida para UTC.
    Aware(DateTime<Utc>),
    /// Sem fuso. No Python, subtrair de um `now` com fuso levanta
    /// `TypeError`, que o chamador trata como data inválida.
    Naive,
    /// `ValueError`.
    Invalid,
}

/// Lê um carimbo do GitHub com a mesma tolerância do Python; `None` quando o
/// Python cairia no `except (TypeError, ValueError)`.
pub fn parse_github_timestamp(value: &str) -> Option<DateTime<Utc>> {
    match py_fromisoformat(&value.replace('Z', "+00:00")) {
        PyIsoDatetime::Aware(instant) => Some(instant),
        PyIsoDatetime::Naive | PyIsoDatetime::Invalid => None,
    }
}

/// Dias inteiros entre dois instantes com o arredondamento de
/// `timedelta.days` do Python (para baixo, também quando negativo).
pub fn python_days(later: DateTime<Utc>, earlier: DateTime<Utc>) -> i64 {
    let delta = later.signed_duration_since(earlier);
    match delta.num_microseconds() {
        Some(micros) => micros.div_euclid(86_400_000_000),
        None => delta.num_seconds().div_euclid(86_400),
    }
}

/// `datetime_fromisoformat()` do CPython 3.12.
pub fn py_fromisoformat(value: &str) -> PyIsoDatetime {
    // _sanitize_isoformat_str: menos de 7 caracteres (code points) é inválido.
    // (Separador substituto — o outro trabalho dela — não existe em `&str`.)
    if value.chars().count() < 7 {
        return PyIsoDatetime::Invalid;
    }
    let b = Bytes(value.as_bytes());
    let len = value.len();

    let Some(separator) = find_isoformat_datetime_separator(&b, len) else {
        return PyIsoDatetime::Invalid;
    };
    let Some((year, month, day)) = parse_isoformat_date(&b, separator) else {
        return PyIsoDatetime::Invalid;
    };

    let mut time = ParsedTime::default();
    if len > separator {
        // O separador pode ser multibyte; o C pula pelo byte inicial.
        let lead = b.at(separator);
        let width = if lead & 0x80 == 0 {
            1
        } else {
            match lead & 0xf0 {
                0xe0 => 3,
                0xf0 => 4,
                _ => 2,
            }
        };
        let start = separator + width;
        let Some(parsed) = len.checked_sub(start).and_then(|dtlen| parse_isoformat_time(&b, start, dtlen)) else {
            return PyIsoDatetime::Invalid;
        };
        time = parsed;
    }

    // tzinfo_from_isoformat_results + new_timezone: deslocamento zero em
    // segundos é UTC (os microssegundos são descartados!); o resto precisa
    // estar estritamente entre -24 h e +24 h.
    let offset_us = match time.offset {
        None => None,
        Some((0, _)) => Some(0),
        Some((seconds, micros)) => {
            let total = i64::from(seconds) * 1_000_000 + i64::from(micros);
            if total.abs() >= 86_400_000_000 {
                return PyIsoDatetime::Invalid;
            }
            Some(total)
        }
    };

    // new_datetime: checagem de faixa de cada campo.
    if !(1..=9999).contains(&year) || time.hour > 23 || time.minute > 59 || time.second > 59 {
        return PyIsoDatetime::Invalid;
    }
    let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
        return PyIsoDatetime::Invalid;
    };
    let clock = NaiveTime::from_hms_micro_opt(time.hour, time.minute, time.second, time.microsecond)
        .expect("campos já checados");
    match offset_us {
        None => PyIsoDatetime::Naive,
        Some(offset) => PyIsoDatetime::Aware((date.and_time(clock) - Duration::microseconds(offset)).and_utc()),
    }
}

/// Os bytes do texto com a leitura do C: fora do fim, lê-se `'\0'`.
struct Bytes<'a>(&'a [u8]);

impl Bytes<'_> {
    fn at(&self, index: usize) -> u8 {
        self.0.get(index).copied().unwrap_or(0)
    }

    /// `parse_digits()`: exatamente `count` dígitos ASCII a partir de `at`.
    fn digits(&self, at: usize, count: usize) -> Option<u32> {
        (at..at + count).try_fold(0_u32, |value, index| {
            let digit = self.at(index).wrapping_sub(b'0');
            (digit <= 9).then(|| value * 10 + u32::from(digit))
        })
    }
}

/// `_find_isoformat_datetime_separator()`. `None` é o `-1` do C (só ocorre
/// em textos que a leitura da data recusaria de qualquer modo).
fn find_isoformat_datetime_separator(b: &Bytes, len: usize) -> Option<usize> {
    if len == 7 {
        return Some(7);
    }
    if b.at(4) == b'-' {
        if b.at(5) == b'W' {
            if len < 8 {
                return None;
            }
            if len > 8 && b.at(8) == b'-' {
                if len == 9 {
                    return None;
                }
                if len > 10 && b.at(10).is_ascii_digit() {
                    return Some(8);
                }
                return Some(10);
            }
            return Some(8);
        }
        return Some(10);
    }
    if b.at(4) == b'W' {
        let mut index = 7;
        while index < len && b.at(index).is_ascii_digit() {
            index += 1;
        }
        if index < 9 {
            return Some(index);
        }
        return Some(if index % 2 == 0 { 7 } else { 8 });
    }
    Some(8)
}

/// `parse_isoformat_date()`: ano, mês e dia ainda sem checagem de faixa
/// (quem checa é o construtor, como no C). Data de semana ISO já sai
/// convertida.
fn parse_isoformat_date(b: &Bytes, len: usize) -> Option<(i32, u32, u32)> {
    let year = b.digits(0, 4)?;
    let mut p = 4;
    let uses_separator = b.at(p) == b'-';
    if uses_separator {
        p += 1;
    }

    if b.at(p) == b'W' {
        p += 1;
        let week = b.digits(p, 2)?;
        p += 2;
        let weekday = if p < len {
            if uses_separator {
                if b.at(p) != b'-' {
                    return None;
                }
                p += 1;
            }
            b.digits(p, 1)?
        } else {
            1
        };
        return iso_to_ymd(year, week, weekday);
    }

    let month = b.digits(p, 2)?;
    p += 2;
    if uses_separator {
        if b.at(p) != b'-' {
            return None;
        }
        p += 1;
    }
    let day = b.digits(p, 2)?;
    Some((i32::try_from(year).ok()?, month, day))
}

/// `iso_to_ymd()` do 3.12: ano ISO fora de 1..=9999, semana inexistente
/// naquele ano ou dia fora de 1..=7 é inválido.
fn iso_to_ymd(iso_year: u32, iso_week: u32, iso_day: u32) -> Option<(i32, u32, u32)> {
    if !(1..=9999).contains(&iso_year) {
        return None;
    }
    let weekday = match iso_day {
        1 => Weekday::Mon,
        2 => Weekday::Tue,
        3 => Weekday::Wed,
        4 => Weekday::Thu,
        5 => Weekday::Fri,
        6 => Weekday::Sat,
        7 => Weekday::Sun,
        _ => return None,
    };
    // A regra da semana 53 do C (1/1 numa quinta, ou quarta em ano bissexto)
    // é a mesma que o chrono aplica.
    let date = NaiveDate::from_isoywd_opt(i32::try_from(iso_year).ok()?, iso_week, weekday)?;
    Some((date.year(), date.month(), date.day()))
}

#[derive(Default)]
struct ParsedTime {
    hour: u32,
    minute: u32,
    second: u32,
    microsecond: u32,
    /// Fuso: (segundos, microssegundos), ambos com o sinal aplicado.
    offset: Option<(i32, i32)>,
}

/// `parse_hh_mm_ss_ff()`: `(hora, minuto, segundo, microssegundo)` e se o
/// texto continua depois (o `return 1` do C). Lê além de `end` como o C lê.
fn parse_hh_mm_ss_ff(b: &Bytes, start: usize, end: usize) -> Option<([u32; 4], bool)> {
    let mut values = [0_u32; 4];
    let mut p = start;
    let mut has_separator = true;
    let mut index = 0;
    while index < 3 {
        values[index] = b.digits(p, 2)?;
        p += 2;
        let c = b.at(p);
        p += 1;
        if index == 0 {
            has_separator = c == b':';
        }
        if p >= end {
            return Some((values, c != 0));
        } else if has_separator && c == b':' {
            index += 1;
            continue;
        } else if c == b'.' || c == b',' {
            break;
        } else if !has_separator {
            p -= 1;
        } else {
            return None; // separador de hora malformado
        }
        index += 1;
    }

    // Fração: até 6 dígitos valem, o resto é pulado.
    // Sempre sobra ao menos um byte aqui (quem chega ao fim retornou acima);
    // o C indexaria `correction[-1]` se não sobrasse.
    let to_parse = end.saturating_sub(p).min(6);
    let fraction = b.digits(p, to_parse)?;
    p += to_parse;
    const CORRECTION: [u32; 6] = [1_000_000, 100_000, 10_000, 1_000, 100, 10];
    values[3] = match to_parse {
        0 => return None,
        6 => fraction,
        n => fraction * CORRECTION[n],
    };
    while b.at(p).is_ascii_digit() {
        p += 1;
    }
    Some((values, b.at(p) != 0))
}

/// `parse_isoformat_time()`: hora e, se houver, fuso.
fn parse_isoformat_time(b: &Bytes, start: usize, dtlen: usize) -> Option<ParsedTime> {
    let end = start + dtlen;
    let mut tz = start;
    loop {
        if matches!(b.at(tz), b'Z' | b'+' | b'-') {
            break;
        }
        tz += 1;
        if tz >= end {
            break;
        }
    }

    let ([hour, minute, second, microsecond], more) = parse_hh_mm_ss_ff(b, start, tz)?;
    let mut time = ParsedTime { hour, minute, second, microsecond, offset: None };
    if tz == end {
        // Sem fuso: sobra no fim é erro.
        return (!more).then_some(time);
    }
    if b.at(tz) == b'Z' {
        time.offset = Some((0, 0));
        return (b.at(tz + 1) == 0).then_some(time);
    }

    let sign = if b.at(tz) == b'-' { -1 } else { 1 };
    let ([tz_hour, tz_minute, tz_second, tz_micro], tz_more) = parse_hh_mm_ss_ff(b, tz + 1, end)?;
    if tz_more {
        return None;
    }
    // Sem checagem de faixa por campo: `+00:60` é uma hora.
    let seconds = i32::try_from(tz_hour * 3600 + tz_minute * 60 + tz_second).ok()?;
    time.offset = Some((sign * seconds, sign * i32::try_from(tz_micro).ok()?));
    Some(time)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(value: &str) -> String {
        match py_fromisoformat(value) {
            PyIsoDatetime::Aware(instant) => instant.format("%Y-%m-%dT%H:%M:%S%.6f").to_string(),
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn formato_do_github() {
        let parsed = parse_github_timestamp("2026-09-20T10:00:00Z").expect("válido");
        assert_eq!("2026-09-20T10:00:00+00:00", parsed.to_rfc3339());
    }

    #[test]
    fn fuso_e_aplicado() {
        assert_eq!("2026-09-20T07:00:00.000000", utc("2026-09-20T10:00:00+03:00"));
    }

    #[test]
    fn peculiaridades_do_cpython() {
        assert_eq!("2026-09-20T10:00:00.500000", utc("2026-09-20T10.5+00:00"), "fração depois da hora");
        assert_eq!("2026-09-20T09:00:00.000000", utc("2026-09-20T10:00:00+00:60"), "minuto 60 no fuso");
        assert_eq!("2026-09-20T10:00:00.000000", utc("2026-09-20T10:00:00 +00:00"), "caractere antes do fuso");
        assert_eq!("2026-09-20T10:00:00.000000", utc("2026-09-20é10:00:00+00:00"), "separador multibyte");
        assert_eq!("2026-09-20T10:00:00.000000", utc("2026-W38-7T10:00:00+00:00"), "semana ISO");
        assert_eq!("Invalid", utc("2026-263T10:00:00+00:00"), "data ordinal não");
        assert_eq!("Invalid", utc("2026-09-20T10:00:00+24:00"));
    }

    #[test]
    fn sem_fuso_e_invalido_como_no_python() {
        assert!(parse_github_timestamp("2026-09-20T10:00:00").is_none());
        assert!(parse_github_timestamp("2026-09-20").is_none());
        assert!(parse_github_timestamp("2026-09-20T10:00:00z").is_none());
        assert_eq!(PyIsoDatetime::Naive, py_fromisoformat("2026-09-20"));
    }

    #[test]
    fn dias_arredondam_para_baixo() {
        let now = parse_github_timestamp("2026-09-25T12:00:00Z").unwrap();
        let later = parse_github_timestamp("2026-09-25T18:00:00Z").unwrap();
        assert_eq!(-1, python_days(now, later), "timedelta(hours=-6).days == -1");
    }
}
