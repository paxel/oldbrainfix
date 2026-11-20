//! Code coloured by its language (obf-1036, ADR 0054), as the Dart
//! core's `highlight.dart` colours it: a grammar is a standard Sublime
//! syntax definition (`.sublime-syntax`), read as data; nothing of it
//! runs. Both cores read the same grammars and colour the same text into
//! the same spans, so they read only a subset of what Sublime Text reads
//! and refuse a grammar beyond it (the Dart core's library comment lists
//! it). Regular expressions are rewritten for fancy-regex so that they
//! mean what they mean to Dart: `\w`, `\d`, `\s` and `\b` ASCII, `.` any
//! character but a line break, `$` before the line's break, `(?i)` for
//! ASCII letters only. Offsets here are UTF-8 bytes.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use fancy_regex::Regex;
use yaml_rust2::{Yaml, YamlLoader};

#[cfg(test)]
mod tests;

/// What a coloured part of code is, which each look draws in a colour of
/// its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodeColour {
    Comment,
    String,
    Number,
    Constant,
    Keyword,
    Operator,
    Type,
    Function,
    Tag,
    Attribute,
    Variable,
}

impl CodeColour {
    pub const ALL: [CodeColour; 11] = [
        CodeColour::Comment,
        CodeColour::String,
        CodeColour::Number,
        CodeColour::Constant,
        CodeColour::Keyword,
        CodeColour::Operator,
        CodeColour::Type,
        CodeColour::Function,
        CodeColour::Tag,
        CodeColour::Attribute,
        CodeColour::Variable,
    ];

    /// Its name as the Dart core's enum names it, for the fixtures.
    pub fn name(self) -> &'static str {
        match self {
            CodeColour::Comment => "comment",
            CodeColour::String => "string",
            CodeColour::Number => "number",
            CodeColour::Constant => "constant",
            CodeColour::Keyword => "keyword",
            CodeColour::Operator => "operator",
            CodeColour::Type => "type",
            CodeColour::Function => "function",
            CodeColour::Tag => "tag",
            CodeColour::Attribute => "attribute",
            CodeColour::Variable => "variable",
        }
    }
}

/// A coloured part of a text, from `start` to `end` in UTF-8 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeSpan {
    pub start: usize,
    pub end: usize,
    pub colour: CodeColour,
}

/// Why a grammar is not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarRefused(pub String);

type Refused<T> = std::result::Result<T, GrammarRefused>;

fn refuse<T>(why: impl Into<String>) -> Refused<T> {
    Err(GrammarRefused(why.into()))
}

/// The scopes that give a colour, by their dotted prefix (as the Dart
/// core's `_colours`).
const COLOURS: &[(&str, CodeColour)] = &[
    ("comment", CodeColour::Comment),
    ("markup.quote", CodeColour::Comment),
    ("string", CodeColour::String),
    ("markup.raw", CodeColour::String),
    ("constant", CodeColour::Constant),
    ("constant.numeric", CodeColour::Number),
    ("variable.language", CodeColour::Constant),
    ("markup.underline.link", CodeColour::Constant),
    ("keyword", CodeColour::Keyword),
    ("storage.modifier", CodeColour::Keyword),
    ("markup.heading", CodeColour::Keyword),
    ("keyword.operator", CodeColour::Operator),
    ("storage.type", CodeColour::Type),
    ("entity.name.type", CodeColour::Type),
    ("entity.name.class", CodeColour::Type),
    ("entity.name.struct", CodeColour::Type),
    ("entity.name.enum", CodeColour::Type),
    ("entity.name.trait", CodeColour::Type),
    ("entity.name.interface", CodeColour::Type),
    ("entity.other.inherited-class", CodeColour::Type),
    ("support.type", CodeColour::Type),
    ("support.class", CodeColour::Type),
    ("entity.name.function", CodeColour::Function),
    ("support.function", CodeColour::Function),
    ("variable.function", CodeColour::Function),
    ("entity.name.tag", CodeColour::Tag),
    ("entity.other.attribute-name", CodeColour::Attribute),
    ("variable.parameter", CodeColour::Variable),
];

/// The colour of one `scope`, by its longest prefix in [`COLOURS`].
pub fn scope_colour(scope: &str) -> Option<CodeColour> {
    let mut name = scope;
    loop {
        if let Some((_, c)) = COLOURS.iter().find(|(p, _)| *p == name) {
            return Some(*c);
        }
        name = name.get(..name.rfind('.')?)?;
    }
}

/// The colour of `scopes`, outermost first: the innermost that has one.
fn colour_of<'a>(scopes: impl IntoIterator<Item = &'a [String]>) -> Option<CodeColour> {
    let mut found = None;
    for list in scopes {
        for s in list {
            found = scope_colour(s).or(found);
        }
    }
    found
}

/// The longest line, and the longest text, that is coloured, in
/// characters; longer ones stay as they are.
pub const MAX_COLOURED_LINE: usize = 5000;
pub const MAX_COLOURED_TEXT: usize = 200_000;
const MAX_STACK: usize = 64;
const MAX_EMPTY: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    None,
    Push,
    Set,
    Pop,
}

/// A context's index in [`Grammar::contexts`].
type Ctx = usize;

#[derive(Debug, Default)]
struct Context {
    meta_scope: Vec<String>,
    meta_content_scope: Vec<String>,
    include_prototype: bool,
    /// Its items as written: a pattern or a context to include.
    items: Vec<Item>,
    patterns: Vec<usize>,
}

#[derive(Debug)]
enum Item {
    Pattern(usize),
    Include(Ctx),
}

#[derive(Debug)]
struct Pattern {
    re: Regex,
    groups: usize,
    scope: Vec<String>,
    captures: HashMap<usize, Vec<String>>,
    action: Action,
    targets: Vec<Ctx>,
}

/// A match: where it lies and where each capture does, in bytes of the
/// line.
#[derive(Debug, Clone)]
struct Found {
    start: usize,
    end: usize,
    groups: Vec<Option<(usize, usize)>>,
}

impl Pattern {
    fn of(source: &str) -> Refused<Pattern> {
        let parsed = CodeRegex::parse(source)?;
        let mut out = String::new();
        emit(&Node::Alt(parsed.root), &mut out);
        let re = fancy_regex::RegexBuilder::new(&out)
            .backtrack_limit(10_000_000)
            .build()
            .map_err(|e| GrammarRefused(format!("the regex {source}: {e}")))?;
        Ok(Pattern {
            re,
            groups: parsed.groups,
            scope: Vec::new(),
            captures: HashMap::new(),
            action: Action::None,
            targets: Vec::new(),
        })
    }

    fn search(&self, line: &str, from: usize) -> Option<Found> {
        // A search that runs out of its backtracking finds nothing.
        let c = self.re.captures_from_pos(line, from).ok()??;
        let m = c.get(0)?;
        let groups = (0..=self.groups)
            .map(|k| c.get(k).map(|g| (g.start(), g.end())))
            .collect();
        Some(Found {
            start: m.start(),
            end: m.end(),
            groups,
        })
    }
}

/// One language's grammar, read from a Sublime syntax definition.
#[derive(Debug)]
pub struct Grammar {
    /// The name it shows, "Rust".
    pub name: String,
    /// Its scope, `source.rust`: what a Code text stores as its language.
    pub scope: String,
    /// The names of its files' extensions, lower case, which are also the
    /// names a fence may give it.
    pub file_extensions: Vec<String>,
    first_line: Option<Pattern>,
    contexts: Vec<Context>,
    patterns: Vec<Pattern>,
    main: Ctx,
}

const TOP_KEYS: [&str; 8] = [
    "name",
    "scope",
    "file_extensions",
    "first_line_match",
    "hidden",
    "version",
    "variables",
    "contexts",
];

fn text<'a>(value: &'a Yaml, what: &str) -> Refused<&'a str> {
    match value {
        Yaml::String(s) => Ok(s),
        _ => refuse(format!("{what} is not a text")),
    }
}

fn scope_list(value: Option<&Yaml>, what: &str) -> Refused<Vec<String>> {
    match value {
        None | Some(Yaml::Null) => Ok(Vec::new()),
        Some(v) => Ok(text(v, what)?
            .split([' ', '\t', '\n'])
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()),
    }
}

fn key(name: &str) -> Yaml {
    Yaml::String(name.to_string())
}

fn hash_keys(h: &yaml_rust2::yaml::Hash) -> impl Iterator<Item = Option<&str>> {
    h.keys().map(|k| match k {
        Yaml::String(s) => Some(s.as_str()),
        _ => None,
    })
}

/// The next `{{name}}` in `text`: what comes before it, the name, and
/// what comes after it; a `{{` without a name after it is part of what
/// comes before.
fn next_variable(text: &str) -> Option<(&str, &str, &str)> {
    let mut from = 0;
    loop {
        let open = from + text.get(from..)?.find("{{")?;
        let after = text.get(open + 2..)?;
        let len = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        let (name, rest) = after.split_at_checked(len)?;
        if len > 0
            && let Some(rest) = rest.strip_prefix("}}")
        {
            return Some((text.get(..open)?, name, rest));
        }
        from = open + 1;
    }
}

/// `{{name}}` in `pattern` replaced by its variable.
fn substitute(pattern: &str, variables: &HashMap<String, String>) -> Refused<String> {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some((before, name, after)) = next_variable(rest) {
        let Some(v) = variables.get(name) else {
            return refuse(format!("no variable {name}"));
        };
        out.push_str(before);
        out.push_str(v);
        rest = after;
    }
    out.push_str(rest);
    Ok(out)
}

fn variables(raw: Option<&Yaml>) -> Refused<HashMap<String, String>> {
    let source: HashMap<String, String> = match raw {
        None | Some(Yaml::Null) => return Ok(HashMap::new()),
        Some(Yaml::Hash(h)) => {
            let mut out = HashMap::new();
            for (k, v) in h {
                let name = text(k, "a variable")?;
                out.insert(
                    name.to_string(),
                    text(v, &format!("variable {name}"))?.to_string(),
                );
            }
            out
        }
        Some(_) => return refuse("variables"),
    };
    fn resolve(
        name: &str,
        depth: usize,
        source: &HashMap<String, String>,
        done: &mut HashMap<String, String>,
    ) -> Refused<String> {
        if let Some(known) = done.get(name) {
            return Ok(known.clone());
        }
        let Some(text) = source.get(name) else {
            return refuse(format!("no variable {name}"));
        };
        if depth > 32 {
            return refuse(format!("variable {name} refers to itself"));
        }
        // The variables it refers to, resolved first.
        let mut names = Vec::new();
        let mut rest = text.as_str();
        while let Some((_, name, after)) = next_variable(rest) {
            names.push(name.to_string());
            rest = after;
        }
        let mut known = HashMap::new();
        for n in names {
            let v = resolve(&n, depth + 1, source, done)?;
            known.insert(n, v);
        }
        let out = substitute(text, &known)?;
        done.insert(name.to_string(), out.clone());
        Ok(out)
    }
    let mut done = HashMap::new();
    let mut names: Vec<&String> = source.keys().collect();
    names.sort();
    for name in names {
        resolve(name, 0, &source, &mut done)?;
    }
    Ok(done)
}

/// Compiles the contexts of one grammar.
struct Compiler<'a> {
    raw: &'a yaml_rust2::yaml::Hash,
    variables: HashMap<String, String>,
    named: HashMap<String, Ctx>,
    contexts: Vec<Context>,
    patterns: Vec<Pattern>,
}

impl Compiler<'_> {
    fn new_context(&mut self) -> Ctx {
        self.contexts.push(Context {
            include_prototype: true,
            ..Context::default()
        });
        self.contexts.len() - 1
    }

    fn context_mut(&mut self, c: Ctx) -> Refused<&mut Context> {
        match self.contexts.get_mut(c) {
            Some(c) => Ok(c),
            None => refuse("a context"),
        }
    }

    fn compile(&mut self) -> Refused<Ctx> {
        let mut names = Vec::new();
        for k in hash_keys(self.raw) {
            let Some(k) = k else {
                return refuse("a context name is not a text");
            };
            names.push(k.to_string());
        }
        if !names.iter().any(|n| n == "main") {
            return refuse("no main");
        }
        for n in &names {
            let c = self.new_context();
            self.named.insert(n.clone(), c);
        }
        let raw = self.raw;
        for (k, v) in raw {
            let name = text(k, "a context name")?;
            let Some(&c) = self.named.get(name) else {
                return refuse("a context");
            };
            self.fill(c, v, name)?;
        }
        let prototype = self.named.get("prototype").copied();
        for c in 0..self.contexts.len() {
            let mut out = Vec::new();
            let mut seen = HashSet::new();
            let with_prototype = self.contexts.get(c).is_some_and(|x| x.include_prototype);
            if let Some(p) = prototype
                && p != c
                && with_prototype
            {
                self.flatten(p, &mut out, &mut seen);
            }
            self.flatten(c, &mut out, &mut seen);
            self.context_mut(c)?.patterns = out;
        }
        match self.named.get("main") {
            Some(&m) => Ok(m),
            None => refuse("no main"),
        }
    }

    fn flatten(&self, c: Ctx, out: &mut Vec<usize>, seen: &mut HashSet<Ctx>) {
        if !seen.insert(c) {
            return;
        }
        let Some(context) = self.contexts.get(c) else {
            return;
        };
        for item in &context.items {
            match item {
                Item::Pattern(p) => out.push(*p),
                Item::Include(i) => self.flatten(*i, out, seen),
            }
        }
    }

    fn fill(&mut self, c: Ctx, items: &Yaml, what: &str) -> Refused<()> {
        let Yaml::Array(items) = items else {
            return refuse(format!("context {what} is no list"));
        };
        let mut first = true;
        for item in items {
            let Yaml::Hash(item) = item else {
                return refuse(format!("an item of {what}"));
            };
            let has = |k: &str| item.contains_key(&key(k));
            let get = |k: &str| item.get(&key(k));
            if has("meta_scope") || has("meta_content_scope") || has("meta_include_prototype") {
                for k in hash_keys(item) {
                    if !matches!(
                        k,
                        Some("meta_scope" | "meta_content_scope" | "meta_include_prototype")
                    ) {
                        return refuse(format!("the key {k:?} in {what}"));
                    }
                }
                if !first {
                    return refuse(format!("a meta item late in {what}"));
                }
                if has("meta_scope") {
                    let s = scope_list(get("meta_scope"), "meta_scope")?;
                    self.context_mut(c)?.meta_scope = s;
                }
                if has("meta_content_scope") {
                    let s = scope_list(get("meta_content_scope"), "meta_content_scope")?;
                    self.context_mut(c)?.meta_content_scope = s;
                }
                if let Some(v) = get("meta_include_prototype") {
                    let Yaml::Boolean(b) = v else {
                        return refuse("meta_include_prototype");
                    };
                    self.context_mut(c)?.include_prototype = *b;
                }
                continue;
            }
            first = false;
            if let Some(include) = get("include") {
                if item.len() != 1 {
                    return refuse(format!("an include in {what}"));
                }
                let name = text(include, "include")?;
                let Some(&target) = self.named.get(name) else {
                    return refuse(format!("no context {name}"));
                };
                self.context_mut(c)?.items.push(Item::Include(target));
                continue;
            }
            let Some(m) = get("match") else {
                return refuse(format!("an item of {what}"));
            };
            for k in hash_keys(item) {
                if !matches!(
                    k,
                    Some("match" | "scope" | "captures" | "push" | "set" | "pop")
                ) {
                    return refuse(format!("the key {k:?} in {what}"));
                }
            }
            let mut pattern = Pattern::of(&substitute(text(m, "match")?, &self.variables)?)?;
            pattern.scope = scope_list(get("scope"), "scope")?;
            match get("captures") {
                None => {}
                Some(Yaml::Hash(caps)) => {
                    for (k, v) in caps {
                        let n = match k {
                            Yaml::Integer(n) => Some(*n),
                            Yaml::String(s) => s.parse::<i64>().ok(),
                            _ => None,
                        };
                        let Some(n) = n
                            .and_then(|n| usize::try_from(n).ok())
                            .filter(|n| *n >= 1 && *n <= pattern.groups)
                        else {
                            return refuse(format!("the capture {k:?}"));
                        };
                        pattern
                            .captures
                            .insert(n, scope_list(Some(v), "a capture")?);
                    }
                }
                Some(_) => return refuse("captures"),
            }
            let actions: Vec<&str> = ["push", "set", "pop"]
                .into_iter()
                .filter(|a| has(a))
                .collect();
            if actions.len() > 1 {
                return refuse(format!("two actions in {what}"));
            }
            match actions.first().copied() {
                None => {}
                Some("pop") => {
                    if !matches!(get("pop"), Some(Yaml::Boolean(true) | Yaml::Integer(1))) {
                        return refuse("pop");
                    }
                    pattern.action = Action::Pop;
                }
                Some(action) => {
                    pattern.action = if action == "push" {
                        Action::Push
                    } else {
                        Action::Set
                    };
                    let Some(target) = get(action) else {
                        return refuse("a push or set");
                    };
                    pattern.targets = self.targets(target, what)?;
                }
            }
            self.patterns.push(pattern);
            let p = self.patterns.len() - 1;
            self.context_mut(c)?.items.push(Item::Pattern(p));
        }
        Ok(())
    }

    fn targets(&mut self, value: &Yaml, what: &str) -> Refused<Vec<Ctx>> {
        match value {
            Yaml::String(name) => match self.named.get(name) {
                Some(&c) => Ok(vec![c]),
                None => refuse(format!("no context {name}")),
            },
            Yaml::Array(list) if !list.is_empty() => {
                if list.iter().all(|v| matches!(v, Yaml::String(_))) {
                    let mut out = Vec::new();
                    for v in list {
                        out.extend(self.targets(v, what)?);
                    }
                    Ok(out)
                } else if list.iter().all(|v| matches!(v, Yaml::Hash(_))) {
                    let c = self.new_context();
                    self.fill(c, value, &format!("{what} (anonymous)"))?;
                    Ok(vec![c])
                } else {
                    refuse(format!("a push or set in {what}"))
                }
            }
            _ => refuse(format!("a push or set in {what}")),
        }
    }
}

impl Grammar {
    /// Reads the grammar `source`; refused when it is no Sublime syntax
    /// definition, or one beyond what the cores read.
    pub fn parse(source: &str) -> Refused<Grammar> {
        let docs = YamlLoader::load_from_str(source)
            .map_err(|e| GrammarRefused(format!("not YAML: {e}")))?;
        let Some(Yaml::Hash(doc)) = docs.first() else {
            return refuse("not a map");
        };
        for k in hash_keys(doc) {
            if !k.is_some_and(|k| TOP_KEYS.contains(&k)) {
                return refuse(format!("the key {k:?}"));
            }
        }
        let get = |k: &str| doc.get(&key(k));
        let name = text(get("name").unwrap_or(&Yaml::Null), "name")?.to_string();
        let scope = text(get("scope").unwrap_or(&Yaml::Null), "scope")?.to_string();
        if name.trim().is_empty() || scope.trim().is_empty() {
            return refuse("no name or scope");
        }
        let mut file_extensions = Vec::new();
        match get("file_extensions") {
            None | Some(Yaml::Null) => {}
            Some(Yaml::Array(list)) => {
                for e in list {
                    file_extensions.push(text(e, "file_extensions")?.to_lowercase());
                }
            }
            Some(_) => return refuse("file_extensions"),
        }
        let variables = variables(get("variables"))?;
        let first_line = match get("first_line_match") {
            None => None,
            Some(v) => Some(Pattern::of(&substitute(
                text(v, "first_line_match")?,
                &variables,
            )?)?),
        };
        let Some(Yaml::Hash(raw)) = get("contexts") else {
            return refuse("no contexts");
        };
        let mut compiler = Compiler {
            raw,
            variables,
            named: HashMap::new(),
            contexts: Vec::new(),
            patterns: Vec::new(),
        };
        let main = compiler.compile()?;
        Ok(Grammar {
            name,
            scope,
            file_extensions,
            first_line,
            contexts: compiler.contexts,
            patterns: compiler.patterns,
            main,
        })
    }

    /// Whether `line` is a first line of this language by its
    /// `first_line_match`.
    pub fn matches_first_line(&self, line: &str) -> bool {
        self.first_line
            .as_ref()
            .is_some_and(|p| p.search(line, 0).is_some())
    }

    /// The coloured parts of `text`, in the order they come, touching
    /// parts of one colour merged; none for a text too long.
    pub fn colour(&self, text: &str) -> Vec<CodeSpan> {
        if text.chars().count() > MAX_COLOURED_TEXT {
            return Vec::new();
        }
        // The colour of each byte, 0 for none: later paint wins.
        let mut painted = vec![0u8; text.len()];
        let mut stack = vec![self.main];
        let mut at = 0;
        while at < text.len() {
            let rest = text.get(at..).unwrap_or_default();
            let end = rest.find('\n').map_or(text.len(), |n| at + n + 1);
            let line = text.get(at..end).unwrap_or_default();
            if line.chars().count() <= MAX_COLOURED_LINE {
                self.line(line, at, &mut stack, &mut painted);
            }
            at = end;
        }
        let mut spans = Vec::new();
        let mut k = 0;
        while k < painted.len() {
            let c = painted.get(k).copied().unwrap_or(0);
            let mut e = k + 1;
            while painted.get(e) == Some(&c) {
                e += 1;
            }
            if let Some(colour) = (c as usize)
                .checked_sub(1)
                .and_then(|i| CodeColour::ALL.get(i))
            {
                spans.push(CodeSpan {
                    start: k,
                    end: e,
                    colour: *colour,
                });
            }
            k = e;
        }
        spans
    }

    fn context(&self, c: Ctx) -> Option<&Context> {
        self.contexts.get(c)
    }

    fn scopes(&self, stack: &[Ctx]) -> Vec<&[String]> {
        let mut out = Vec::new();
        for c in stack {
            if let Some(c) = self.context(*c) {
                out.push(c.meta_scope.as_slice());
                out.push(c.meta_content_scope.as_slice());
            }
        }
        out
    }

    /// Colours one `line` that starts at `offset` of the text, with the
    /// contexts of `stack`, which it leaves as the line ends.
    fn line(&self, line: &str, offset: usize, stack: &mut Vec<Ctx>, painted: &mut [u8]) {
        let mut paint = |start: usize, end: usize, colour: Option<CodeColour>| {
            if let Some(colour) = colour
                && let Some(range) = painted.get_mut(offset + start..offset + end)
            {
                let v = CodeColour::ALL
                    .iter()
                    .position(|c| *c == colour)
                    .unwrap_or(0) as u8
                    + 1;
                range.fill(v);
            }
        };
        let mut pos = 0;
        let mut empties = 0;
        let mut cached_for: Option<Ctx> = None;
        let mut cache: Vec<Option<Found>> = Vec::new();
        let mut searched_from: Vec<Option<usize>> = Vec::new();
        loop {
            let Some(&top) = stack.last() else {
                return;
            };
            let Some(context) = self.context(top) else {
                return;
            };
            if cached_for != Some(top) {
                cached_for = Some(top);
                cache = vec![None; context.patterns.len()];
                searched_from = vec![None; context.patterns.len()];
            }
            let mut best: Option<(Found, usize)> = None;
            for (i, &p) in context.patterns.iter().enumerate() {
                let Some(pattern) = self.patterns.get(p) else {
                    continue;
                };
                let (Some(slot), Some(from)) = (cache.get_mut(i), searched_from.get_mut(i)) else {
                    continue;
                };
                let valid =
                    from.is_some_and(|f| f <= pos) && slot.as_ref().is_none_or(|f| f.start >= pos);
                if !valid {
                    *slot = pattern.search(line, pos);
                    *from = Some(pos);
                }
                let Some(found) = slot.as_ref() else {
                    continue;
                };
                if found.start == found.end && pattern.action == Action::None {
                    continue;
                }
                if best.as_ref().is_none_or(|(b, _)| found.start < b.start) {
                    best = Some((found.clone(), p));
                }
            }
            let content = colour_of(self.scopes(stack));
            let Some((best, p)) = best else {
                paint(pos, line.len(), content);
                return;
            };
            let Some(pattern) = self.patterns.get(p) else {
                return;
            };
            paint(pos, best.start, content);
            if best.end == pos {
                empties += 1;
                if empties > MAX_EMPTY {
                    return;
                }
            } else {
                empties = 0;
            }
            let mut action = pattern.action;
            let grows = match action {
                Action::Push => pattern.targets.len(),
                Action::Set => pattern.targets.len().saturating_sub(1),
                _ => 0,
            };
            if stack.len() + grows > MAX_STACK {
                action = Action::None;
            }
            if action == Action::Pop && stack.len() == 1 {
                action = Action::None;
            }
            let below = stack.get(..stack.len() - 1).unwrap_or_default();
            // The scopes the match's own text has.
            let mut base: Vec<&[String]> = match action {
                Action::None | Action::Push => self.scopes(stack),
                Action::Set => self.scopes(below),
                Action::Pop => {
                    let mut b = self.scopes(below);
                    b.push(context.meta_scope.as_slice());
                    b
                }
            };
            if matches!(action, Action::Push | Action::Set) {
                for t in &pattern.targets {
                    if let Some(c) = self.context(*t) {
                        base.push(c.meta_scope.as_slice());
                    }
                }
            }
            base.push(pattern.scope.as_slice());
            paint(best.start, best.end, colour_of(base.iter().copied()));
            for (k, g) in best.groups.iter().enumerate().skip(1) {
                let (Some((s, e)), Some(cap)) = (g, pattern.captures.get(&k)) else {
                    continue;
                };
                if s >= e || colour_of([cap.as_slice()]).is_none() {
                    continue;
                }
                let mut with = base.clone();
                with.push(cap.as_slice());
                paint(*s, *e, colour_of(with));
            }
            match action {
                Action::None => {}
                Action::Push => stack.extend(pattern.targets.iter().copied()),
                Action::Set => {
                    stack.pop();
                    stack.extend(pattern.targets.iter().copied());
                }
                Action::Pop => {
                    stack.pop();
                }
            }
            pos = best.end;
        }
    }
}

/// The grammars the app ships with (obf-1036), in the order the
/// language picker shows them and a guess tries them, as the Dart core's
/// `tool/gen_grammars.dart` lists them.
pub const BUILT_IN_SOURCES: [&str; 15] = [
    include_str!("../../../../assets/grammars/shell.sublime-syntax"),
    include_str!("../../../../assets/grammars/python.sublime-syntax"),
    include_str!("../../../../assets/grammars/javascript.sublime-syntax"),
    include_str!("../../../../assets/grammars/rust.sublime-syntax"),
    include_str!("../../../../assets/grammars/dart.sublime-syntax"),
    include_str!("../../../../assets/grammars/kotlin.sublime-syntax"),
    include_str!("../../../../assets/grammars/java.sublime-syntax"),
    include_str!("../../../../assets/grammars/c.sublime-syntax"),
    include_str!("../../../../assets/grammars/go.sublime-syntax"),
    include_str!("../../../../assets/grammars/sql.sublime-syntax"),
    include_str!("../../../../assets/grammars/json.sublime-syntax"),
    include_str!("../../../../assets/grammars/yaml.sublime-syntax"),
    include_str!("../../../../assets/grammars/xml.sublime-syntax"),
    include_str!("../../../../assets/grammars/css.sublime-syntax"),
    include_str!("../../../../assets/grammars/markdown.sublime-syntax"),
];

/// The grammars this app reads, in the order a guess tries them.
#[derive(Debug)]
pub struct Grammars {
    pub all: Vec<Grammar>,
}

impl Grammars {
    /// The grammars the app ships with, read once.
    pub fn built_in() -> &'static Grammars {
        static BUILT_IN: OnceLock<Grammars> = OnceLock::new();
        BUILT_IN.get_or_init(|| Grammars {
            all: BUILT_IN_SOURCES
                .iter()
                .filter_map(|s| Grammar::parse(s).ok())
                .collect(),
        })
    }

    /// The grammar whose scope is `scope`, a Code text's language; none
    /// for an empty one and for one not known here.
    pub fn by_scope(&self, scope: Option<&str>) -> Option<&Grammar> {
        let scope = scope.filter(|s| !s.is_empty())?;
        self.all.iter().find(|g| g.scope == scope)
    }

    /// The grammar a fence's `language` names: by one of its file
    /// extensions or its name, compared in lower case.
    pub fn for_fence(&self, language: &str) -> Option<&Grammar> {
        let name = language.trim().to_lowercase();
        if name.is_empty() {
            return None;
        }
        self.all
            .iter()
            .find(|g| g.file_extensions.contains(&name) || g.name.to_lowercase() == name)
    }

    /// The grammar a pasted `text` is guessed to be in: the first whose
    /// `first_line_match` its first line that is not blank matches.
    pub fn guess(&self, text: &str) -> Option<&Grammar> {
        let line = text.split('\n').find(|l| !l.trim().is_empty())?;
        let line = line.strip_suffix('\r').unwrap_or(line);
        self.all.iter().find(|g| g.matches_first_line(line))
    }
}

// The regular expressions of the subset, read into a tree (as the Dart
// core's `_RegexParser`) and written out for fancy-regex.

#[derive(Debug, Clone)]
enum Node {
    Seq(Vec<Node>),
    Alt(Vec<Node>),
    Group(GroupKind, Box<Node>),
    Quant {
        atom: Box<Node>,
        min: u32,
        max: Option<u32>,
        lazy: bool,
    },
    /// A set of code points, both ends of each range included.
    Set {
        negated: bool,
        ranges: Vec<(u32, u32)>,
    },
    Anchor(AnchorKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupKind {
    Capture,
    Plain,
    Ahead,
    NotAhead,
    Behind,
    NotBehind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnchorKind {
    Start,
    End,
    Word,
    NotWord,
}

fn has_capture(n: &Node) -> bool {
    match n {
        Node::Seq(t) | Node::Alt(t) => t.iter().any(has_capture),
        Node::Group(kind, body) => *kind == GroupKind::Capture || has_capture(body),
        Node::Quant { atom, .. } => has_capture(atom),
        Node::Set { .. } | Node::Anchor(_) => false,
    }
}

const WORD: [(u32, u32); 4] = [(0x30, 0x39), (0x41, 0x5A), (0x5F, 0x5F), (0x61, 0x7A)];
const DIGIT: [(u32, u32); 1] = [(0x30, 0x39)];
const SPACE: [(u32, u32); 2] = [(0x09, 0x0D), (0x20, 0x20)];

/// A regular expression of the subset, read: its tree (the alternatives
/// of its top level) and how many captures it has.
#[derive(Debug)]
pub struct CodeRegex {
    root: Vec<Node>,
    pub groups: usize,
}

impl CodeRegex {
    /// Reads `source`; refused for anything beyond the subset.
    pub fn parse(source: &str) -> Refused<CodeRegex> {
        let mut p = Parser {
            s: source.chars().map(u32::from).collect(),
            source,
            i: 0,
            extended: false,
            fold: false,
            groups: 0,
        };
        p.parse()
    }
}

struct Parser<'a> {
    s: Vec<u32>,
    source: &'a str,
    i: usize,
    extended: bool,
    fold: bool,
    groups: usize,
}

impl Parser<'_> {
    fn refuse<T>(&self, what: &str) -> Refused<T> {
        refuse(format!("the regex {}: {what}", self.source))
    }

    fn peek(&self) -> Option<u32> {
        self.s.get(self.i).copied()
    }

    fn peek_at(&self, k: usize) -> Option<u32> {
        self.s.get(self.i + k).copied()
    }

    fn at(&self, text: &str) -> bool {
        text.chars()
            .enumerate()
            .all(|(k, c)| self.peek_at(k) == Some(u32::from(c)))
    }

    fn parse(&mut self) -> Refused<CodeRegex> {
        // Leading flags: (?x), (?i), (?xi), (?ix), in any number.
        while self.i + 2 < self.s.len() && self.at("(?") {
            let mut k = self.i + 2;
            let (mut x, mut ic) = (false, false);
            while let Some(c) = self.s.get(k).copied().filter(|c| *c == 0x78 || *c == 0x69) {
                if c == 0x78 {
                    x = true;
                } else {
                    ic = true;
                }
                k += 1;
            }
            if k == self.i + 2 || self.s.get(k) != Some(&0x29) {
                break;
            }
            self.extended |= x;
            self.fold |= ic;
            self.i = k + 1;
        }
        let root = self.alt()?;
        if self.i < self.s.len() {
            return self.refuse("a ) too many");
        }
        Ok(CodeRegex {
            root,
            groups: self.groups,
        })
    }

    fn skip(&mut self) {
        if !self.extended {
            return;
        }
        while let Some(c) = self.peek() {
            if c == 0x20 || (0x09..=0x0D).contains(&c) {
                self.i += 1;
            } else if c == 0x23 {
                while self.peek().is_some_and(|c| c != 0x0A) {
                    self.i += 1;
                }
            } else {
                return;
            }
        }
    }

    fn alt(&mut self) -> Refused<Vec<Node>> {
        let mut alts = vec![self.seq()?];
        while self.peek() == Some(0x7C) {
            self.i += 1;
            alts.push(self.seq()?);
        }
        Ok(alts)
    }

    fn seq(&mut self) -> Refused<Node> {
        let mut terms = Vec::new();
        loop {
            self.skip();
            match self.peek() {
                None | Some(0x7C | 0x29) => break,
                Some(_) => {}
            }
            let atom = self.atom()?;
            self.skip();
            terms.push(self.quantified(atom)?);
        }
        Ok(Node::Seq(terms))
    }

    fn quantified(&mut self, atom: Node) -> Refused<Node> {
        let (min, max) = match self.peek() {
            Some(0x2A) => {
                self.i += 1;
                (0, None)
            }
            Some(0x2B) => {
                self.i += 1;
                (1, None)
            }
            Some(0x3F) => {
                self.i += 1;
                (0, Some(1))
            }
            Some(0x7B) => match self.braces()? {
                Some(q) => q,
                None => return Ok(atom),
            },
            _ => return Ok(atom),
        };
        let mut lazy = false;
        if self.peek() == Some(0x3F) {
            lazy = true;
            self.i += 1;
        }
        if matches!(self.peek(), Some(0x2B | 0x2A | 0x3F)) {
            return self.refuse("a possessive or doubled quantifier");
        }
        if self.peek() == Some(0x7B) && self.brace_ahead()? {
            return self.refuse("a doubled quantifier");
        }
        if matches!(atom, Node::Anchor(_)) {
            return self.refuse("a quantifier on an assertion");
        }
        if let Node::Group(kind, _) = &atom {
            if !matches!(kind, GroupKind::Capture | GroupKind::Plain) {
                return self.refuse("a quantifier on a lookaround");
            }
            if has_capture(&atom) {
                return self.refuse("a capture in a quantified group");
            }
        }
        Ok(Node::Quant {
            atom: Box::new(atom),
            min,
            max,
            lazy,
        })
    }

    fn brace_ahead(&mut self) -> Refused<bool> {
        let keep = self.i;
        let q = self.braces()?;
        self.i = keep;
        Ok(q.is_some())
    }

    /// `{n}`, `{n,}` or `{n,m}`, read; none, and the place as it was, for
    /// a `{` that is a literal.
    #[allow(clippy::type_complexity)]
    fn braces(&mut self) -> Refused<Option<(u32, Option<u32>)>> {
        let keep = self.i;
        self.i += 1;
        if self.peek() == Some(0x2C) {
            return self.refuse("a quantifier {,n}");
        }
        let number = |p: &mut Self| -> Option<u32> {
            let from = p.i;
            let mut v: u32 = 0;
            while let Some(d) = p.peek().filter(|c| (0x30..=0x39).contains(c)) {
                v = v * 10 + (d - 0x30);
                p.i += 1;
                if p.i - from > 4 {
                    return None;
                }
            }
            (p.i > from).then_some(v)
        };
        let Some(min) = number(self) else {
            self.i = keep;
            return Ok(None);
        };
        let mut max = Some(min);
        if self.peek() == Some(0x2C) {
            self.i += 1;
            max = number(self);
        }
        if self.peek() != Some(0x7D) {
            self.i = keep;
            return Ok(None);
        }
        self.i += 1;
        if max.is_some_and(|m| m < min) {
            return self.refuse("a quantifier {n,m} with m < n");
        }
        Ok(Some((min, max)))
    }

    fn literal(&self, c: u32) -> Node {
        Node::Set {
            negated: false,
            ranges: self.folded(vec![(c, c)]),
        }
    }

    fn folded(&self, ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
        if !self.fold {
            return ranges;
        }
        let mut out = ranges.clone();
        for (lo, hi) in ranges {
            let (a, b) = (lo.max(0x61), hi.min(0x7A));
            if a <= b {
                out.push((a - 0x20, b - 0x20));
            }
            let (x, y) = (lo.max(0x41), hi.min(0x5A));
            if x <= y {
                out.push((x + 0x20, y + 0x20));
            }
        }
        out
    }

    fn atom(&mut self) -> Refused<Node> {
        let Some(c) = self.peek() else {
            return self.refuse("nothing to read");
        };
        match c {
            0x28 => self.group(),
            0x5B => self.class(),
            0x2E => {
                self.i += 1;
                Ok(Node::Set {
                    negated: true,
                    ranges: vec![(0x0A, 0x0A)],
                })
            }
            0x5E => {
                self.i += 1;
                Ok(Node::Anchor(AnchorKind::Start))
            }
            0x24 => {
                self.i += 1;
                Ok(Node::Anchor(AnchorKind::End))
            }
            0x5C => self.escape(),
            0x2A | 0x2B | 0x3F => self.refuse("a quantifier without anything before it"),
            0x7B => {
                if self.brace_ahead()? {
                    return self.refuse("a quantifier without anything before it");
                }
                self.i += 1;
                Ok(self.literal(c))
            }
            _ => {
                self.i += 1;
                Ok(self.literal(c))
            }
        }
    }

    fn group(&mut self) -> Refused<Node> {
        self.i += 1;
        let mut kind = GroupKind::Capture;
        if self.peek() == Some(0x3F) {
            let (k, len) = if self.at("?:") {
                (GroupKind::Plain, 2)
            } else if self.at("?=") {
                (GroupKind::Ahead, 2)
            } else if self.at("?!") {
                (GroupKind::NotAhead, 2)
            } else if self.at("?<=") {
                (GroupKind::Behind, 3)
            } else if self.at("?<!") {
                (GroupKind::NotBehind, 3)
            } else {
                return self.refuse("a group (? of a kind not read");
            };
            kind = k;
            self.i += len;
        } else {
            self.groups += 1;
        }
        let body = Node::Alt(self.alt()?);
        if self.peek() != Some(0x29) {
            return self.refuse("a ( not closed");
        }
        self.i += 1;
        if !matches!(kind, GroupKind::Capture | GroupKind::Plain) && has_capture(&body) {
            return self.refuse("a capture in a lookaround");
        }
        Ok(Node::Group(kind, Box::new(body)))
    }

    fn escape(&mut self) -> Refused<Node> {
        self.i += 1;
        let Some(c) = self.peek() else {
            return self.refuse("a \\ at the end");
        };
        let set = |negated: bool, ranges: &[(u32, u32)]| Node::Set {
            negated,
            ranges: ranges.to_vec(),
        };
        let node = match c {
            0x77 => set(false, &WORD),
            0x57 => set(true, &WORD),
            0x64 => set(false, &DIGIT),
            0x44 => set(true, &DIGIT),
            0x73 => set(false, &SPACE),
            0x53 => set(true, &SPACE),
            0x62 => Node::Anchor(AnchorKind::Word),
            0x42 => Node::Anchor(AnchorKind::NotWord),
            _ => {
                let c = self.char()?;
                return Ok(self.literal(c));
            }
        };
        self.i += 1;
        Ok(node)
    }

    /// The character an escape (after its `\`) stands for.
    fn char(&mut self) -> Refused<u32> {
        let Some(c) = self.peek() else {
            return self.refuse("a \\ at the end");
        };
        self.i += 1;
        match c {
            0x74 => return Ok(0x09),
            0x6E => return Ok(0x0A),
            0x72 => return Ok(0x0D),
            0x66 => return Ok(0x0C),
            0x76 => return Ok(0x0B),
            0x65 => return Ok(0x1B),
            0x61 => return Ok(0x07),
            0x78 => {
                if self.peek() == Some(0x7B) {
                    self.i += 1;
                    let from = self.i;
                    let mut v: u32 = 0;
                    while let Some(d) = self.peek().and_then(hex) {
                        v = v.saturating_mul(16).saturating_add(d);
                        self.i += 1;
                    }
                    if self.i == from || self.i - from > 6 || self.peek() != Some(0x7D) {
                        return self.refuse("a \\x{…} not closed");
                    }
                    self.i += 1;
                    return self.scalar(v);
                }
                let v = self.hex_digits(2)?;
                return self.scalar(v);
            }
            0x75 => {
                let v = self.hex_digits(4)?;
                return self.scalar(v);
            }
            _ => {}
        }
        if c < 0x80 && char::from_u32(c).is_some_and(|c| c.is_ascii_alphanumeric()) {
            return self.refuse(&format!(
                "the escape \\{}",
                char::from_u32(c).unwrap_or('?')
            ));
        }
        Ok(c)
    }

    fn hex_digits(&mut self, n: usize) -> Refused<u32> {
        let mut v = 0;
        for _ in 0..n {
            let Some(d) = self.peek().and_then(hex) else {
                return self.refuse("a short hex escape");
            };
            v = v * 16 + d;
            self.i += 1;
        }
        Ok(v)
    }

    fn scalar(&self, v: u32) -> Refused<u32> {
        if v > 0x10FFFF || (0xD800..=0xDFFF).contains(&v) {
            return self.refuse(&format!("no character {v:x}"));
        }
        Ok(v)
    }

    fn class(&mut self) -> Refused<Node> {
        self.i += 1;
        let mut negated = false;
        if self.peek() == Some(0x5E) {
            negated = true;
            self.i += 1;
        }
        let mut ranges = Vec::new();
        let mut first = true;
        loop {
            let Some(c) = self.peek() else {
                return self.refuse("a [ not closed");
            };
            if c == 0x5D && !first {
                self.i += 1;
                break;
            }
            first = false;
            if c == 0x5B {
                return self.refuse("a [ in a class");
            }
            if self.at("&&") {
                return self.refuse("&& in a class");
            }
            let lo = if c == 0x5C {
                self.i += 1;
                let Some(e) = self.peek() else {
                    return self.refuse("a \\ at the end");
                };
                let set: Option<&[(u32, u32)]> = match e {
                    0x77 => Some(&WORD),
                    0x64 => Some(&DIGIT),
                    0x73 => Some(&SPACE),
                    _ => None,
                };
                if let Some(set) = set {
                    self.i += 1;
                    ranges.extend_from_slice(set);
                    continue;
                }
                if matches!(e, 0x57 | 0x44 | 0x53 | 0x62 | 0x42) {
                    return self.refuse("a class escape in a class");
                }
                self.char()?
            } else {
                self.i += 1;
                c
            };
            if self.peek() == Some(0x2D) && self.peek_at(1).is_some_and(|n| n != 0x5D) {
                self.i += 1;
                let hi = match self.peek() {
                    Some(0x5C) => {
                        self.i += 1;
                        if matches!(
                            self.peek(),
                            Some(0x77 | 0x64 | 0x73 | 0x57 | 0x44 | 0x53 | 0x62 | 0x42)
                        ) {
                            return self.refuse("a range to a class");
                        }
                        self.char()?
                    }
                    Some(0x5B) => return self.refuse("a [ in a class"),
                    Some(h) => {
                        self.i += 1;
                        h
                    }
                    None => return self.refuse("a [ not closed"),
                };
                if hi < lo {
                    return self.refuse("a range backwards");
                }
                ranges.push((lo, hi));
            } else {
                ranges.push((lo, lo));
            }
        }
        Ok(Node::Set {
            negated,
            ranges: self.folded(ranges),
        })
    }
}

fn hex(c: u32) -> Option<u32> {
    char::from_u32(c)?.to_digit(16)
}

const W: &str = r"[A-Za-z0-9_]";

/// A character as fancy-regex reads it alike in a class and out of one:
/// an ASCII letter or digit as itself, anything else as `\x{…}`.
fn lit(c: u32, out: &mut String) {
    match char::from_u32(c) {
        Some(ch) if ch.is_ascii_alphanumeric() => out.push(ch),
        _ => out.push_str(&format!("\\x{{{c:X}}}")),
    }
}

fn emit(n: &Node, out: &mut String) {
    match n {
        Node::Seq(terms) => terms.iter().for_each(|t| emit(t, out)),
        Node::Alt(alts) => {
            for (k, a) in alts.iter().enumerate() {
                if k > 0 {
                    out.push('|');
                }
                emit(a, out);
            }
        }
        Node::Group(kind, body) => {
            out.push_str(match kind {
                GroupKind::Capture => "(",
                GroupKind::Plain => "(?:",
                GroupKind::Ahead => "(?=",
                GroupKind::NotAhead => "(?!",
                GroupKind::Behind => "(?<=",
                GroupKind::NotBehind => "(?<!",
            });
            emit(body, out);
            out.push(')');
        }
        Node::Quant {
            atom,
            min,
            max,
            lazy,
        } => {
            emit(atom, out);
            match (min, max) {
                (0, None) => out.push('*'),
                (1, None) => out.push('+'),
                (0, Some(1)) => out.push('?'),
                (a, None) => out.push_str(&format!("{{{a},}}")),
                (a, Some(b)) if a == b => out.push_str(&format!("{{{a}}}")),
                (a, Some(b)) => out.push_str(&format!("{{{a},{b}}}")),
            }
            if *lazy {
                out.push('?');
            }
        }
        Node::Set { negated, ranges } => {
            if let [(lo, hi)] = ranges.as_slice()
                && lo == hi
                && !negated
            {
                lit(*lo, out);
                return;
            }
            out.push_str(if *negated { "[^" } else { "[" });
            for (lo, hi) in ranges {
                lit(*lo, out);
                if hi != lo {
                    out.push('-');
                    lit(*hi, out);
                }
            }
            out.push(']');
        }
        Node::Anchor(kind) => match kind {
            AnchorKind::Start => out.push('^'),
            AnchorKind::End => out.push_str(r"(?![^\x{A}])"),
            AnchorKind::Word => out.push_str(&format!("(?:(?<={W})(?!{W})|(?<!{W})(?={W}))")),
            AnchorKind::NotWord => out.push_str(&format!("(?:(?<={W})(?={W})|(?<!{W})(?!{W}))")),
        },
    }
}
