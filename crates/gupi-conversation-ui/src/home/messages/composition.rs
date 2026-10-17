//! Strict composition of recorded patches. Unknown original ranges stay sparse;
//! neither matching nor serialization invents their contents.
use std::collections::BTreeMap;
use std::fmt::Write;
use std::rc::Rc;

use super::edited_files::group_path;

#[cfg(test)]
mod tests;

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_LINES: usize = 100_000;
const MAX_PATCHES: usize = 2048;
const MAX_WORK: usize = 8_000_000;
// Leave ample room for checked shifts, including on 32-bit targets.
const UNKNOWN_END: usize = usize::MAX / 4;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Line {
    text: Rc<str>,
    newline: bool,
}
#[derive(Clone)]
struct Row {
    kind: u8,
    line: Line,
}
struct Hunk {
    new: usize,
    old_count: usize,
    new_count: usize,
    rows: Vec<Row>,
}
#[derive(Clone)]
enum Piece {
    Original { start: usize, len: usize },
    Added(Line),
}
impl Piece {
    fn len(&self) -> usize {
        match self {
            Self::Original { len, .. } => *len,
            Self::Added(_) => 1,
        }
    }
}
struct Document {
    known: BTreeMap<usize, Line>,
    pieces: Vec<Piece>,
    original_len: usize,
    work: usize,
}

/// `None` means the recorded changes cancel exactly.
pub(super) fn compose(path: &str, cwd: &str, patches: &[&str]) -> Result<Option<String>, ()> {
    check_budget(patches)?;
    let mut document = Document::new(UNKNOWN_END);
    for patch in patches {
        document.patch(&parse(path, cwd, patch)?)?;
    }
    document.serialize(path)
}

/// Apply edits to a successful write using Pi's per-edit BOM/newline protocol.
pub(super) fn apply(path: &str, cwd: &str, content: &str, patches: &[&str]) -> Result<String, ()> {
    check_budget(patches)?;
    if content.len() > MAX_BYTES {
        return Err(());
    }
    let mut content = content.to_owned();
    let mut copied_bytes = 0usize;
    for patch in patches {
        copied_bytes = copied_bytes.checked_add(content.len()).ok_or(())?;
        if copied_bytes > MAX_BYTES * 8 {
            return Err(());
        }
        let (bom, text) = content
            .strip_prefix('\u{feff}')
            .map_or(("", content.as_str()), |text| ("\u{feff}", text));
        let crlf = text
            .find('\n')
            .is_some_and(|ix| ix > 0 && text.as_bytes()[ix - 1] == b'\r');
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let mut lines = text_lines(&normalized);
        if lines.len() > MAX_LINES {
            return Err(());
        }
        apply_hunks(&mut lines, &parse(path, cwd, patch)?)?;
        let mut next = String::from(bom);
        for line in lines {
            next.push_str(&line.text);
            if line.newline {
                next.push_str(if crlf { "\r\n" } else { "\n" });
            }
        }
        if next.len() > MAX_BYTES {
            return Err(());
        }
        content = next;
    }
    Ok(content)
}

/// Fully known writes need no sparse provenance map. The parser has checked the
/// old/new coordinate relationship, so earlier hunks leave the next old range
/// exactly at its new-side start. All matching remains positional and exact.
fn apply_hunks(lines: &mut Vec<Line>, hunks: &[Hunk]) -> Result<(), ()> {
    let mut work = 0usize;
    for hunk in hunks {
        let end = hunk.new.checked_add(hunk.old_count).ok_or(())?;
        let old = lines.get(hunk.new..end).ok_or(())?;
        if !old.iter().eq(hunk
            .rows
            .iter()
            .filter(|row| row.kind != b'+')
            .map(|row| &row.line))
        {
            return Err(());
        }
        if hunk
            .rows
            .iter()
            .any(|row| row.kind != b'-' && !row.line.newline)
            && end != lines.len()
        {
            return Err(());
        }
        let next_len = lines
            .len()
            .checked_sub(hunk.old_count)
            .and_then(|len| len.checked_add(hunk.new_count))
            .ok_or(())?;
        if next_len > MAX_LINES {
            return Err(());
        }
        let shifted = if hunk.old_count == hunk.new_count {
            0
        } else {
            lines.len() - end
        };
        work = work
            .checked_add(hunk.rows.len())
            .and_then(|work| work.checked_add(shifted))
            .ok_or(())?;
        if work > MAX_WORK {
            return Err(());
        }
        lines.splice(
            hunk.new..end,
            hunk.rows
                .iter()
                .filter(|row| row.kind != b'-')
                .map(|row| row.line.clone()),
        );
    }
    if lines
        .iter()
        .take(lines.len().saturating_sub(1))
        .any(|line| !line.newline)
    {
        return Err(());
    }
    Ok(())
}

fn check_budget(patches: &[&str]) -> Result<(), ()> {
    if patches.len() > MAX_PATCHES
        || patches
            .iter()
            .try_fold(0usize, |sum, patch| sum.checked_add(patch.len()))
            .ok_or(())?
            > MAX_BYTES
    {
        Err(())
    } else {
        Ok(())
    }
}
fn text_lines(text: &str) -> Vec<Line> {
    text.split_inclusive('\n')
        .map(|line| Line {
            text: line.strip_suffix('\n').unwrap_or(line).into(),
            newline: line.ends_with('\n'),
        })
        .collect()
}

fn header_path(header: &str) -> Result<String, ()> {
    if header.starts_with('"') {
        // Git's C-quoted paths include octal UTF-8 bytes, unlike JSON strings.
        let mut bytes = Vec::new();
        let mut chars = header.as_bytes().iter().copied().skip(1);
        while let Some(byte) = chars.next() {
            match byte {
                b'"' => {
                    let rest: Vec<_> = chars.collect();
                    if !rest.is_empty() && rest.first() != Some(&b'\t') {
                        return Err(());
                    }
                    return String::from_utf8(bytes).map_err(|_| ());
                }
                b'\\' => {
                    let escaped = chars.next().ok_or(())?;
                    bytes.push(match escaped {
                        b'\\' | b'"' => escaped,
                        b't' => b'\t',
                        b'n' => b'\n',
                        b'r' => b'\r',
                        b'a' => 7,
                        b'b' => 8,
                        b'f' => 12,
                        b'v' => 11,
                        b'0'..=b'3' => {
                            let b = chars
                                .next()
                                .filter(|b| matches!(b, b'0'..=b'7'))
                                .ok_or(())?;
                            let c = chars
                                .next()
                                .filter(|b| matches!(b, b'0'..=b'7'))
                                .ok_or(())?;
                            (escaped - b'0') * 64 + (b - b'0') * 8 + (c - b'0')
                        }
                        _ => return Err(()),
                    });
                }
                _ => bytes.push(byte),
            }
        }
        Err(())
    } else {
        Ok(header.split('\t').next().ok_or(())?.to_owned())
    }
}
fn range(value: &str) -> Result<(usize, usize), ()> {
    let (start, count) = value.split_once(',').unwrap_or((value, "1"));
    if start.is_empty()
        || count.is_empty()
        || !start.bytes().all(|b| b.is_ascii_digit())
        || !count.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(());
    }
    let start: usize = start.parse().map_err(|_| ())?;
    let count: usize = count.parse().map_err(|_| ())?;
    let start = if count == 0 {
        start
    } else {
        start.checked_sub(1).ok_or(())?
    };
    if start.checked_add(count).ok_or(())? >= UNKNOWN_END || count > MAX_LINES {
        return Err(());
    }
    Ok((start, count))
}
fn parse(path: &str, cwd: &str, patch: &str) -> Result<Vec<Hunk>, ()> {
    // Pi emits a framing LF even when the source has no final newline. An
    // unterminated record is ambiguous with truncated transport.
    if !patch.ends_with('\n') {
        return Err(());
    }
    // A CR on both file headers identifies CRLF *transport*. With LF headers,
    // CR on body lines is source content and must survive unchanged.
    let raw: Vec<_> = patch.split_terminator('\n').collect();
    let header_ix = raw
        .iter()
        .position(|line| line.starts_with("--- "))
        .ok_or(())?;
    if header_ix > 2 {
        return Err(());
    }
    let transport = raw[header_ix].ends_with('\r');
    let lines: Vec<_> = raw
        .iter()
        .map(|line| {
            if transport {
                line.strip_suffix('\r').ok_or(())
            } else {
                Ok(*line)
            }
        })
        .collect::<Result<_, _>>()?;
    // Only ordinary git index metadata is supported; mode/rename/binary records
    // have semantics this projection cannot reconstruct.
    for (ix, line) in lines[..header_ix].iter().enumerate() {
        if !(ix == 0 && line.starts_with("diff --git ") || line.starts_with("index ") && ix == 1) {
            return Err(());
        }
    }
    let old = header_path(lines[header_ix].strip_prefix("--- ").ok_or(())?)?;
    let new = header_path(
        lines
            .get(header_ix + 1)
            .ok_or(())?
            .strip_prefix("+++ ")
            .ok_or(())?,
    )?;
    if header_ix > 0 {
        let git_paths = lines[0].strip_prefix("diff --git ").ok_or(())?;
        let quoted_old = quote_path(&old);
        let quoted_new = quote_path(&new);
        let matching_paths = [old.as_str(), quoted_old.as_str()].iter().any(|before| {
            [new.as_str(), quoted_new.as_str()].iter().any(|after| {
                git_paths
                    .strip_prefix(before)
                    .and_then(|rest| rest.strip_prefix(' '))
                    == Some(*after)
            })
        });
        if !matching_paths {
            return Err(());
        }
        if header_ix == 2 {
            let index = lines[1].strip_prefix("index ").ok_or(())?;
            let mut fields = index.split(' ');
            let (before, after) = fields.next().ok_or(())?.split_once("..").ok_or(())?;
            if before.is_empty()
                || after.is_empty()
                || !before
                    .bytes()
                    .chain(after.bytes())
                    .all(|byte| byte.is_ascii_hexdigit())
                || fields
                    .next()
                    .is_some_and(|mode| !matches!(mode, "100644" | "100755"))
                || fields.next().is_some()
            {
                return Err(());
            }
        }
    }
    let expected = group_path(path, cwd);
    let direct = group_path(&old, cwd) == expected && group_path(&new, cwd) == expected;
    let git = old
        .strip_prefix("a/")
        .zip(new.strip_prefix("b/"))
        .is_some_and(|(old, new)| {
            group_path(old, cwd) == expected && group_path(new, cwd) == expected
        });
    if expected.is_empty() || old == "/dev/null" || new == "/dev/null" || !(direct || git) {
        return Err(());
    }
    let mut ix = header_ix + 2;
    let mut hunks = Vec::new();
    let mut old_end = 0usize;
    let mut new_end = 0usize;
    let mut line_count = 0usize;
    while ix < lines.len() {
        let header = lines[ix].strip_prefix("@@ -").ok_or(())?;
        let (coordinates, _) = header.split_once(" @@").ok_or(())?;
        let (old_range, new_range) = coordinates.split_once(" +").ok_or(())?;
        let (old, old_count) = range(old_range)?;
        let (new, new_count) = range(new_range)?;
        if old < old_end
            || new < new_end
            || old - old_end != new - new_end
            || old_count + new_count == 0
        {
            return Err(());
        }
        old_end = old + old_count;
        new_end = new + new_count;
        ix += 1;
        let mut rows: Vec<Row> = Vec::new();
        let (mut old_seen, mut new_seen) = (0usize, 0usize);
        let (mut old_eof, mut new_eof) = (false, false);
        while ix < lines.len() && !lines[ix].starts_with("@@ ") {
            let body = lines[ix];
            if body == "\\ No newline at end of file" {
                let previous = rows.last_mut().ok_or(())?;
                if !previous.line.newline {
                    return Err(());
                }
                previous.line.newline = false;
                if previous.kind != b'+' {
                    old_eof = true;
                }
                if previous.kind != b'-' {
                    new_eof = true;
                }
            } else {
                let kind = *body.as_bytes().first().ok_or(())?;
                if !matches!(kind, b' ' | b'+' | b'-') {
                    return Err(());
                }
                if kind != b'+' {
                    if old_eof {
                        return Err(());
                    }
                    old_seen += 1;
                }
                if kind != b'-' {
                    if new_eof {
                        return Err(());
                    }
                    new_seen += 1;
                }
                rows.push(Row {
                    kind,
                    line: Line {
                        text: body[1..].into(),
                        newline: true,
                    },
                });
                line_count += 1;
                if old_seen > old_count || new_seen > new_count || line_count > MAX_LINES {
                    return Err(());
                }
            }
            ix += 1;
        }
        if old_seen != old_count || new_seen != new_count {
            return Err(());
        }
        hunks.push(Hunk {
            new,
            old_count,
            new_count,
            rows,
        });
    }
    Ok(hunks)
}

impl Document {
    fn new(len: usize) -> Self {
        Self {
            known: BTreeMap::new(),
            pieces: if len == 0 {
                vec![]
            } else {
                vec![Piece::Original { start: 0, len }]
            },
            original_len: len,
            work: 0,
        }
    }
    fn charge(&mut self, amount: usize) -> Result<(), ()> {
        self.work = self.work.checked_add(amount).ok_or(())?;
        if self.work > MAX_WORK {
            Err(())
        } else {
            Ok(())
        }
    }
    fn split(&mut self, position: usize) -> Result<usize, ()> {
        self.charge(self.pieces.len())?;
        let mut offset = 0usize;
        for ix in 0..self.pieces.len() {
            if offset == position {
                return Ok(ix);
            }
            let end = offset.checked_add(self.pieces[ix].len()).ok_or(())?;
            if position < end {
                let Piece::Original { start, len } = self.pieces[ix] else {
                    return Err(());
                };
                let left = position - offset;
                self.pieces[ix] = Piece::Original { start, len: left };
                self.pieces.insert(
                    ix + 1,
                    Piece::Original {
                        start: start + left,
                        len: len - left,
                    },
                );
                return Ok(ix + 1);
            }
            offset = end;
        }
        if offset == position {
            Ok(self.pieces.len())
        } else {
            Err(())
        }
    }
    fn eof(&mut self, position: usize) -> Result<(), ()> {
        let ix = self.split(position)?;
        let mut original_end = self.original_len;
        for piece in &self.pieces[ix..] {
            match piece {
                Piece::Original { start, len }
                    if self.known.range(*start..start + len).next().is_none() =>
                {
                    original_end = original_end.min(*start)
                }
                _ => return Err(()),
            }
        }
        self.pieces.truncate(ix);
        self.original_len = original_end;
        Ok(())
    }
    fn patch(&mut self, hunks: &[Hunk]) -> Result<(), ()> {
        for hunk in hunks {
            // Prior hunks have already shifted current coordinates to new-side
            // coordinates; the parser verified both sides' intervening gaps.
            let start = hunk.new;
            if hunk
                .rows
                .iter()
                .any(|row| row.kind != b'+' && !row.line.newline)
            {
                self.eof(start.checked_add(hunk.old_count).ok_or(())?)?;
            }
            let first = self.split(start)?;
            let last = self.split(start.checked_add(hunk.old_count).ok_or(())?)?;
            let mut originals = Vec::with_capacity(hunk.old_count);
            for piece in &self.pieces[first..last] {
                match piece {
                    Piece::Original { start, len } => originals.extend(
                        (*start..start + len).map(|start| Piece::Original { start, len: 1 }),
                    ),
                    Piece::Added(line) => originals.push(Piece::Added(line.clone())),
                }
            }
            let mut originals = originals.into_iter();
            let mut replacement = Vec::with_capacity(hunk.new_count);
            for row in &hunk.rows {
                if row.kind == b'+' {
                    replacement.push(Piece::Added(row.line.clone()));
                    continue;
                }
                let original = originals.next().ok_or(())?;
                match &original {
                    Piece::Original { start, .. } => {
                        if let Some(known) = self.known.get(start) {
                            if known != &row.line {
                                return Err(());
                            }
                        } else {
                            self.known.insert(*start, row.line.clone());
                        }
                    }
                    Piece::Added(line) if line != &row.line => return Err(()),
                    _ => {}
                }
                if row.kind == b' ' {
                    replacement.push(original);
                }
            }
            self.pieces.splice(first..last, replacement);
            if hunk
                .rows
                .iter()
                .any(|row| row.kind != b'-' && !row.line.newline)
            {
                self.eof(start.checked_add(hunk.new_count).ok_or(())?)?;
            }
            self.charge(hunk.rows.len())?;
            if self.pieces.len() > MAX_LINES || self.known.len() > MAX_LINES {
                return Err(());
            }
        }
        // A previous EOF cannot silently become an interior unterminated line.
        let mut ended = false;
        for element in self.elements()? {
            if ended {
                return Err(());
            }
            if let Element::Text(line) = element {
                ended = !line.newline;
            }
        }
        Ok(())
    }
    fn elements(&self) -> Result<Vec<Element>, ()> {
        let mut result = Vec::new();
        for piece in &self.pieces {
            match piece {
                Piece::Added(line) => result.push(Element::Text(line.clone())),
                Piece::Original { start, len } => {
                    let mut cursor = *start;
                    for (&position, line) in self.known.range(*start..start + len) {
                        gap(&mut result, cursor, position - cursor);
                        result.push(Element::Text(line.clone()));
                        cursor = position + 1;
                    }
                    gap(&mut result, cursor, start + len - cursor);
                }
            }
            if result.len() > MAX_LINES * 2 {
                return Err(());
            }
        }
        Ok(result)
    }
    fn serialize(&mut self, path: &str) -> Result<Option<String>, ()> {
        let mut old = Vec::new();
        let mut cursor = 0usize;
        for (&position, line) in &self.known {
            gap(&mut old, cursor, position - cursor);
            old.push(Element::Text(line.clone()));
            cursor = position + 1;
        }
        if cursor > self.original_len {
            return Err(());
        }
        gap(&mut old, cursor, self.original_len - cursor);
        let mut new = self.elements()?;
        let (mut old_ix, mut new_ix) = (0usize, 0usize);
        let (mut old_position, mut new_position) = (0usize, 0usize);
        let mut result = String::new();
        loop {
            let old_start = old_ix;
            let new_start = new_ix;
            while old_ix < old.len() && matches!(old[old_ix], Element::Text(_)) {
                old_ix += 1;
            }
            while new_ix < new.len() && matches!(new[new_ix], Element::Text(_)) {
                new_ix += 1;
            }
            let old_lines = old[old_start..old_ix]
                .iter()
                .map(Element::line)
                .collect::<Vec<_>>();
            let new_lines = new[new_start..new_ix]
                .iter()
                .map(Element::line)
                .collect::<Vec<_>>();
            let rows = self.difference(&old_lines, &new_lines)?;
            serialize_hunks(&mut result, &rows, old_position, new_position)?;
            old_position = old_position.checked_add(old_lines.len()).ok_or(())?;
            new_position = new_position.checked_add(new_lines.len()).ok_or(())?;
            match (old.get(old_ix), new.get(new_ix)) {
                (None, None) => break,
                (Some(Element::Gap(a, len)), Some(Element::Gap(b, other))) if a == b => {
                    // An insertion can split an entirely unknown original gap.
                    // Consume its common prefix without allocating unknown lines.
                    let (a, len, other) = (*a, *len, *other);
                    let shared = len.min(other);
                    old_position = old_position.checked_add(shared).ok_or(())?;
                    new_position = new_position.checked_add(shared).ok_or(())?;
                    if len == shared {
                        old_ix += 1;
                    } else {
                        old[old_ix] = Element::Gap(a + shared, len - shared);
                    }
                    if other == shared {
                        new_ix += 1;
                    } else {
                        new[new_ix] = Element::Gap(a + shared, other - shared);
                    }
                }
                _ => return Err(()),
            }
        }
        if result.is_empty() {
            Ok(None)
        } else {
            let path = quote_path(path);
            Ok(Some(format!("--- {path}\n+++ {path}\n{result}")))
        }
    }
    fn difference(&mut self, old: &[&Line], new: &[&Line]) -> Result<Vec<Row>, ()> {
        let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(new[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let a = &old[prefix..old.len() - suffix];
        let b = &new[prefix..new.len() - suffix];
        let width = b.len() + 1;
        let cells = (a.len() + 1).checked_mul(width).ok_or(())?;
        self.charge(cells)?;
        let mut lcs = vec![0u32; cells];
        for i in (0..a.len()).rev() {
            for j in (0..b.len()).rev() {
                lcs[i * width + j] = if a[i] == b[j] {
                    lcs[(i + 1) * width + j + 1] + 1
                } else {
                    lcs[(i + 1) * width + j].max(lcs[i * width + j + 1])
                };
            }
        }
        let mut rows: Vec<_> = old[..prefix]
            .iter()
            .map(|line| Row {
                kind: b' ',
                line: (*line).clone(),
            })
            .collect();
        let (mut i, mut j) = (0, 0);
        while i < a.len() || j < b.len() {
            let (kind, line) = if i < a.len() && j < b.len() && a[i] == b[j] {
                i += 1;
                j += 1;
                (b' ', a[i - 1])
            } else if i < a.len()
                && (j == b.len() || lcs[(i + 1) * width + j] >= lcs[i * width + j + 1])
            {
                i += 1;
                (b'-', a[i - 1])
            } else {
                j += 1;
                (b'+', b[j - 1])
            };
            rows.push(Row {
                kind,
                line: line.clone(),
            });
        }
        rows.extend(old[old.len() - suffix..].iter().map(|line| Row {
            kind: b' ',
            line: (*line).clone(),
        }));
        Ok(rows)
    }
}

enum Element {
    Gap(usize, usize),
    Text(Line),
}
impl Element {
    fn line(&self) -> &Line {
        match self {
            Self::Text(line) => line,
            Self::Gap(..) => unreachable!(),
        }
    }
}
fn gap(elements: &mut Vec<Element>, start: usize, len: usize) {
    if len == 0 {
        return;
    }
    if let Some(Element::Gap(previous, previous_len)) = elements.last_mut()
        && *previous + *previous_len == start
    {
        *previous_len += len;
    } else {
        elements.push(Element::Gap(start, len));
    }
}
fn quote_path(path: &str) -> String {
    if !path
        .bytes()
        .any(|byte| !(33..=126).contains(&byte) || matches!(byte, b'"' | b'\\'))
    {
        return path.to_owned();
    }
    let mut result = String::from("\"");
    for byte in path.bytes() {
        match byte {
            b'"' => result.push_str("\\\""),
            b'\\' => result.push_str("\\\\"),
            b'\t' => result.push_str("\\t"),
            b'\n' => result.push_str("\\n"),
            b'\r' => result.push_str("\\r"),
            32..=126 => result.push(char::from(byte)),
            _ => {
                let _ = write!(result, "\\{byte:03o}");
            }
        }
    }
    result.push('"');
    result
}
fn serialize_hunks(
    result: &mut String,
    rows: &[Row],
    mut old: usize,
    mut new: usize,
) -> Result<(), ()> {
    let mut cursor = 0;
    while let Some(change) = (cursor..rows.len()).find(|&ix| rows[ix].kind != b' ') {
        let start = change.saturating_sub(4).max(cursor);
        for row in &rows[cursor..start] {
            old += usize::from(row.kind != b'+');
            new += usize::from(row.kind != b'-');
        }
        let mut end = change + 1;
        let mut context: usize = 0;
        while end < rows.len() {
            if rows[end].kind == b' ' {
                context += 1;
                if context > 8 {
                    end -= 4;
                    break;
                }
            } else {
                context = 0;
            }
            end += 1;
        }
        if end == rows.len() {
            end -= context.saturating_sub(4);
        }
        let selected = &rows[start..end];
        let old_count = selected.iter().filter(|row| row.kind != b'+').count();
        let new_count = selected.iter().filter(|row| row.kind != b'-').count();
        writeln!(
            result,
            "@@ -{},{} +{},{} @@",
            old + usize::from(old_count > 0),
            old_count,
            new + usize::from(new_count > 0),
            new_count
        )
        .map_err(|_| ())?;
        for row in selected {
            result.push(char::from(row.kind));
            result.push_str(&row.line.text);
            result.push('\n');
            if !row.line.newline {
                result.push_str("\\ No newline at end of file\n");
            }
        }
        if result.len() > MAX_BYTES {
            return Err(());
        }
        old += old_count;
        new += new_count;
        cursor = end;
    }
    Ok(())
}
