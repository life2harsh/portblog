#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Backspace,
    Tab,
    BackTab,
    PageUp,
    PageDown,
    Home,
    End,
    Interrupt,
}

pub fn parse(bytes: &[u8]) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            0x1b => {
                let rest = &bytes[index + 1..];
                let (key, used) = escape(rest);
                keys.push(key);
                index += 1 + used;
                continue;
            }
            0x03 | 0x04 => keys.push(Key::Interrupt),
            b'\r' | b'\n' => keys.push(Key::Enter),
            b'\t' => keys.push(Key::Tab),
            0x7f | 0x08 => keys.push(Key::Backspace),
            0x00..=0x1f => {}
            _ => {
                let width = utf8_width(byte);
                if let Some(text) = bytes
                    .get(index..index + width)
                    .and_then(|slice| std::str::from_utf8(slice).ok())
                {
                    keys.extend(text.chars().map(Key::Char));
                }
                index += width;
                continue;
            }
        }
        index += 1;
    }
    keys
}

fn escape(rest: &[u8]) -> (Key, usize) {
    match rest {
        [b'[' | b'O', b'A', ..] => (Key::Up, 2),
        [b'[' | b'O', b'B', ..] => (Key::Down, 2),
        [b'[' | b'O', b'C', ..] => (Key::Right, 2),
        [b'[' | b'O', b'D', ..] => (Key::Left, 2),
        [b'[' | b'O', b'H', ..] => (Key::Home, 2),
        [b'[' | b'O', b'F', ..] => (Key::End, 2),
        [b'[', b'Z', ..] => (Key::BackTab, 2),
        [b'[', digit, b'~', ..] => (
            match digit {
                b'1' | b'7' => Key::Home,
                b'4' | b'8' => Key::End,
                b'5' => Key::PageUp,
                b'6' => Key::PageDown,
                _ => Key::Escape,
            },
            3,
        ),
        [b'[', ..] => {
            let used = rest
                .iter()
                .skip(1)
                .position(|byte| (0x40..=0x7e).contains(byte))
                .map_or(rest.len(), |end| end + 2);
            (Key::Escape, used)
        }
        _ => (Key::Escape, 0),
    }
}

fn utf8_width(lead: u8) -> usize {
    match lead {
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_control() {
        assert_eq!(
            parse(b"jk\r"),
            vec![Key::Char('j'), Key::Char('k'), Key::Enter]
        );
        assert_eq!(parse(&[0x03]), vec![Key::Interrupt]);
        assert_eq!(parse(&[0x7f, b'\t']), vec![Key::Backspace, Key::Tab]);
    }

    #[test]
    fn escape_sequences() {
        assert_eq!(
            parse(b"\x1b[A\x1b[B\x1bOC\x1b[D"),
            vec![Key::Up, Key::Down, Key::Right, Key::Left]
        );
        assert_eq!(
            parse(b"\x1b[5~\x1b[6~\x1b[Z"),
            vec![Key::PageUp, Key::PageDown, Key::BackTab]
        );
        assert_eq!(parse(b"\x1b"), vec![Key::Escape]);
        assert_eq!(parse(b"\x1b[1;5Cq"), vec![Key::Escape, Key::Char('q')]);
    }

    #[test]
    fn unicode() {
        assert_eq!(parse("é".as_bytes()), vec![Key::Char('é')]);
    }
}
