//! O que o Python diz quando lê um arquivo de texto — sem ler nada.
//!
//! `Path.read_text(encoding="utf-8")` decodifica o arquivo inteiro de uma vez
//! e traduz as quebras de linha (`\r\n` e `\r` viram `\n`). Quando falha, a
//! mensagem é a do `OSError` (`[Errno 2] No such file or directory: '…'`) ou
//! a do `UnicodeDecodeError` (`'utf-8' codec can't decode byte 0xff in
//! position 3: invalid start byte`). Este módulo converte; quem lê o disco é
//! o chamador — o crate continua sem I/O.

use crate::monitor::PyException;
use crate::pyrepr::py_repr_str;

/// Quebras universais do modo texto: `\r\n` e `\r` viram `\n`.
pub fn universal_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// `bytes.decode("utf-8")` com a mensagem do CPython quando falha.
pub fn decode_utf8(bytes: &[u8]) -> Result<String, PyException> {
    let error = match std::str::from_utf8(bytes) {
        Ok(text) => return Ok(text.to_string()),
        Err(error) => error,
    };
    let start = error.valid_up_to();
    let (end, reason) = match error.error_len() {
        None => (bytes.len(), "unexpected end of data"),
        // Um byte só: ou ele não começa sequência nenhuma, ou começa e o
        // seguinte não continua (o CPython aponta o primeiro nos dois casos).
        Some(1) if matches!(bytes[start], 0xC2..=0xF4) => (start + 1, "invalid continuation byte"),
        Some(1) => (start + 1, "invalid start byte"),
        Some(length) => (start + length, "invalid continuation byte"),
    };
    let what = if end - start == 1 {
        format!("byte 0x{:02x} in position {start}", bytes[start])
    } else {
        format!("bytes in position {start}-{}", end - 1)
    };
    Err(PyException::new("UnicodeDecodeError", format!("'utf-8' codec can't decode {what}: {reason}")))
}

/// As classes de `OSError` que [`os_error`] devolve.
const OS_ERRORS: [&str; 7] = [
    "OSError",
    "PermissionError",
    "FileNotFoundError",
    "InterruptedError",
    "FileExistsError",
    "NotADirectoryError",
    "IsADirectoryError",
];

/// `isinstance(exc, OSError)`.
pub fn is_os_error(exception: &PyException) -> bool {
    OS_ERRORS.contains(&exception.kind)
}

/// O `OSError` que o Python levantaria para `error` ao abrir `path`: a
/// subclasse pelo `errno` e o texto `[Errno N] strerror: 'path'`.
pub fn os_error(error: &std::io::Error, path: &str) -> PyException {
    let Some(errno) = error.raw_os_error() else {
        return PyException::new("OSError", format!("{error}: {}", py_repr_str(path)));
    };
    let kind = match errno {
        1 | 13 => "PermissionError",
        2 => "FileNotFoundError",
        4 => "InterruptedError",
        17 => "FileExistsError",
        20 => "NotADirectoryError",
        21 => "IsADirectoryError",
        _ => "OSError",
    };
    // O Display do Rust é "strerror (os error N)": o mesmo strerror da libc.
    let text = error.to_string();
    let strerror = text.strip_suffix(&format!(" (os error {errno})")).unwrap_or(&text);
    PyException::new(kind, format!("[Errno {errno}] {strerror}: {}", py_repr_str(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mensagens_do_decodificador() {
        let message = |bytes: &[u8]| decode_utf8(bytes).unwrap_err().message;
        assert_eq!("'utf-8' codec can't decode byte 0xff in position 0: invalid start byte", message(b"\xff"));
        assert_eq!(
            "'utf-8' codec can't decode bytes in position 2-3: invalid continuation byte",
            message(b"ab\xe2\x82x")
        );
        assert_eq!("'utf-8' codec can't decode byte 0xe2 in position 0: invalid continuation byte", message(b"\xe2x"));
        assert_eq!("'utf-8' codec can't decode bytes in position 3-4: unexpected end of data", message(b"abc\xe2\x82"));
        assert_eq!("a\nb\nc\n", universal_newlines("a\r\nb\rc\n"));
    }

    #[test]
    fn oserror_do_python() {
        let error = os_error(&std::io::Error::from_raw_os_error(2), "/raiz/it's");
        assert_eq!("FileNotFoundError", error.kind);
        assert_eq!("[Errno 2] No such file or directory: \"/raiz/it's\"", error.message);
    }
}
