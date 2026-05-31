#![allow(
    clippy::double_must_use,
    clippy::mutable_key_type,
    clippy::question_mark
)]

use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    convert::TryFrom,
    str::FromStr,
};

use homotopy_common::tree::Node;
use homotopy_core::{
    common::{Boundary, Direction},
    signature::Invertibility,
    Bias, Diagram, Diagram0, DiagramN, Generator, Orientation,
};
use homotopy_graphics::style::{Color, VertexShape};
use homotopy_model::proof::{
    generators::GeneratorInfo, Metadata, ProofState, Signature, SignatureItem, Workspace,
};
use serde::{Deserialize, Serialize};

pub mod presets;

/// Names from the homotopy.io paper/default editor model that the DSL builds on.
///
/// The readable `cell`/`struct`/`schema` syntax is sugar for ordinary signature
/// diagrams and proof-state actions. The `actions [...]` source form below
/// exposes the same `proof::Action` replay format used by the point-and-click
/// editor, so source can represent anything the default modes can perform.
pub mod spec {
    pub use homotopy_core::{
        common::{BoundaryPath, SliceIndex},
        rewrite::Cone,
        Boundary, Cospan, Diagram, Diagram0, DiagramN, Direction, Generator, Height, Rewrite,
        Rewrite0, RewriteN,
    };
    pub use homotopy_model::proof::{
        homotopy::{Contract, Expand, Homotopy},
        Action, AttachOption, ProofState, Workspace,
    };
}

const COLORS: &[&str] = &[
    "#2980b9", "#c0392b", "#f39c12", "#8e44ad", "#27ae60", "#f1c40f", "#6b7280", "#111827",
];
const MAX_PROOF_SEARCH_DEPTH: usize = 6;
const MAX_PROOF_SEARCH_NODES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    const fn join(self, other: Self) -> Self {
        Self {
            start: self.start,
            end: other.end,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    fn error(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ast {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stmt {
    Metadata {
        key: MetadataKey,
        value: String,
        span: Span,
    },
    Cell(CellDecl),
    Proof(ProofDecl),
    Folder(FolderDecl),
    Declaration(DeclarationDecl),
    Use(UseDecl),
    Show {
        expr: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataKey {
    Title,
    Author,
    Abstract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellDecl {
    pub name: String,
    pub source: Option<Expr>,
    pub target: Option<Expr>,
    pub invertibility: Invertibility,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarationDecl {
    pub kind: DeclarationKind,
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclarationKind {
    Schema,
    Struct,
    Macro,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderDecl {
    pub name: String,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofDecl {
    pub name: String,
    pub source: Expr,
    pub target: Expr,
    pub invertibility: Invertibility,
    pub steps: Vec<ProofStep>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStep {
    Attach {
        expr: Expr,
        span: Span,
    },
    Contract {
        bias: Option<ContractionBias>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractionBias {
    Lower,
    Higher,
    Same,
}

impl From<ContractionBias> for Bias {
    fn from(value: ContractionBias) -> Self {
        match value {
            ContractionBias::Lower => Self::Lower,
            ContractionBias::Higher => Self::Higher,
            ContractionBias::Same => Self::Same,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: ParamType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamType {
    Cell(usize),
    Structure(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseDecl {
    pub declaration: String,
    pub args: Vec<String>,
    pub alias: String,
    pub bindings: Vec<UseBinding>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseBinding {
    pub field: String,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    Name {
        name: String,
        span: Span,
    },
    Identity {
        expr: Box<Expr>,
        span: Span,
    },
    Inverse {
        expr: Box<Expr>,
        span: Span,
    },
    Contract {
        expr: Box<Expr>,
        bias: Option<ContractionBias>,
        span: Span,
    },
    Compose {
        terms: Vec<Expr>,
        span: Span,
    },
}

impl Expr {
    fn span(&self) -> Span {
        match self {
            Self::Name { span, .. }
            | Self::Identity { span, .. }
            | Self::Inverse { span, .. }
            | Self::Contract { span, .. }
            | Self::Compose { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    pub title: Option<String>,
    pub author: Option<String>,
    pub abstr: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompileResult {
    pub ast: Option<Ast>,
    pub proof: Option<ProofState>,
    pub diagnostics: Vec<Diagnostic>,
    pub symbols: Vec<SymbolInfo>,
    pub selected: Option<String>,
}

impl CompileResult {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error)
            && self.proof.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub dimension: usize,
    pub generator: Generator,
}

#[derive(Debug, Clone)]
struct Symbol {
    info: SymbolInfo,
    diagram: Diagram,
    invertibility: Invertibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Ident(String),
    String(String),
    Number(usize),
    Keyword(&'static str),
    Colon,
    Semicolon,
    Comma,
    Dot,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Lt,
    Gt,
    Star,
    Equals,
    Arrow,
    DoubleArrow,
    Eof,
}

#[must_use]
pub fn parse(source: &str) -> Result<Ast, Vec<Diagnostic>> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.lex();
    if !lexer.diagnostics.is_empty() {
        return Err(lexer.diagnostics);
    }

    let mut parser = Parser::new(tokens);
    let ast = parser.parse_program();
    if parser.diagnostics.is_empty() {
        Ok(ast)
    } else {
        Err(parser.diagnostics)
    }
}

#[must_use]
pub fn compile(source: &str, options: CompileOptions) -> CompileResult {
    if let Some(result) = compile_action_source(source, &options) {
        return result;
    }

    let ast = match parse(source) {
        Ok(ast) => ast,
        Err(diagnostics) => {
            return CompileResult {
                ast: None,
                proof: None,
                diagnostics,
                symbols: Vec::new(),
                selected: None,
            }
        }
    };

    let mut compiler = Compiler::new(options);
    compiler.compile_ast(&ast);
    let proof = compiler.finish();

    CompileResult {
        ast: Some(ast),
        proof,
        diagnostics: compiler.diagnostics,
        symbols: compiler
            .symbols
            .values()
            .map(|symbol| symbol.info.clone())
            .collect(),
        selected: compiler.selected,
    }
}

fn compile_action_source(source: &str, options: &CompileOptions) -> Option<CompileResult> {
    let source = source.trim();
    let json = source
        .strip_prefix("actions")
        .map(str::trim_start)
        .or_else(|| source.strip_prefix("paper").map(str::trim_start))
        .or_else(|| source.starts_with('[').then_some(source))?;
    let json = json.strip_suffix(';').unwrap_or(json).trim();

    let actions = match serde_json::from_str::<Vec<homotopy_model::proof::Action>>(json) {
        Ok(actions) => actions,
        Err(vector_error) => {
            match serde_json::from_str::<(bool, Vec<homotopy_model::proof::Action>)>(json) {
                Ok((_, actions)) => actions,
                Err(tuple_error) => {
                    return Some(CompileResult {
                        ast: None,
                        proof: None,
                        diagnostics: vec![Diagnostic::error(
                            format!(
                                "could not parse paper action list: {vector_error}; {tuple_error}"
                            ),
                            Span::new(0, source.len()),
                        )],
                        symbols: Vec::new(),
                        selected: None,
                    });
                }
            }
        }
    };

    let mut proof = ProofState {
        metadata: Metadata {
            title: options.title.clone(),
            author: options.author.clone(),
            abstr: options.abstr.clone(),
        },
        ..Default::default()
    };
    let mut diagnostics = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        match proof.update(action) {
            Ok(updated) => {
                if !updated {
                    diagnostics.push(Diagnostic::error(
                        format!("paper action {index} had no effect"),
                        Span::new(0, source.len()),
                    ));
                }
            }
            Err(error) => diagnostics.push(Diagnostic::error(
                format!("paper action {index} failed: {error}"),
                Span::new(0, source.len()),
            )),
        }
    }

    let symbols = proof
        .signature
        .iter()
        .map(|info| SymbolInfo {
            name: info.name.clone(),
            dimension: info.diagram.dimension(),
            generator: info.generator,
        })
        .collect();
    let proof = diagnostics
        .iter()
        .all(|diagnostic| diagnostic.severity != Severity::Error)
        .then_some(proof);

    Some(CompileResult {
        ast: None,
        proof,
        diagnostics,
        symbols,
        selected: None,
    })
}

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    cursor: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            cursor: 0,
            diagnostics: Vec::new(),
        }
    }

    fn lex(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_ws_and_comments();
            let start = self.cursor;
            let Some(byte) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: Span::new(self.cursor, self.cursor),
                });
                return tokens;
            };

            let token = match byte {
                b':' => self.single(TokenKind::Colon),
                b';' => self.single(TokenKind::Semicolon),
                b',' => self.single(TokenKind::Comma),
                b'.' => self.single(TokenKind::Dot),
                b'(' => self.single(TokenKind::LParen),
                b')' => self.single(TokenKind::RParen),
                b'{' => self.single(TokenKind::LBrace),
                b'}' => self.single(TokenKind::RBrace),
                b'<' if self.peek_next() == Some(b'-') && self.peek_offset(2) == Some(b'>') => {
                    self.cursor += 3;
                    Token {
                        kind: TokenKind::DoubleArrow,
                        span: Span::new(start, self.cursor),
                    }
                }
                b'<' => self.single(TokenKind::Lt),
                b'>' => self.single(TokenKind::Gt),
                b'*' => self.single(TokenKind::Star),
                b'=' => self.single(TokenKind::Equals),
                b'-' if self.peek_next() == Some(b'>') => {
                    self.cursor += 2;
                    Token {
                        kind: TokenKind::Arrow,
                        span: Span::new(start, self.cursor),
                    }
                }
                b'"' => self.string(),
                b'0'..=b'9' => self.number(),
                b if is_ident_start(b) => self.ident_or_keyword(),
                _ => {
                    self.cursor += 1;
                    self.diagnostics.push(Diagnostic::error(
                        format!("unexpected character `{}`", byte as char),
                        Span::new(start, self.cursor),
                    ));
                    continue;
                }
            };
            tokens.push(token);
        }
    }

    fn single(&mut self, kind: TokenKind) -> Token {
        let start = self.cursor;
        self.cursor += 1;
        Token {
            kind,
            span: Span::new(start, self.cursor),
        }
    }

    fn string(&mut self) -> Token {
        let start = self.cursor;
        self.cursor += 1;
        let mut value = String::new();
        while let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.cursor += 1;
                    return Token {
                        kind: TokenKind::String(value),
                        span: Span::new(start, self.cursor),
                    };
                }
                b'\\' if self.peek_next() == Some(b'"') => {
                    self.cursor += 2;
                    value.push('"');
                }
                _ => {
                    value.push(byte as char);
                    self.cursor += 1;
                }
            }
        }

        self.diagnostics.push(Diagnostic::error(
            "unterminated string literal",
            Span::new(start, self.cursor),
        ));
        Token {
            kind: TokenKind::String(value),
            span: Span::new(start, self.cursor),
        }
    }

    fn number(&mut self) -> Token {
        let start = self.cursor;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.cursor += 1;
        }
        let value = self.source[start..self.cursor].parse().unwrap_or_default();
        Token {
            kind: TokenKind::Number(value),
            span: Span::new(start, self.cursor),
        }
    }

    fn ident_or_keyword(&mut self) -> Token {
        let start = self.cursor;
        self.cursor += 1;
        while self.peek().is_some_and(is_ident_continue) {
            self.cursor += 1;
        }
        let ident = &self.source[start..self.cursor];
        let kind = match ident {
            "cell" => TokenKind::Keyword("cell"),
            "prove" => TokenKind::Keyword("prove"),
            "construct" => TokenKind::Keyword("construct"),
            "attach" => TokenKind::Keyword("attach"),
            "contract" => TokenKind::Keyword("contract"),
            "folder" => TokenKind::Keyword("folder"),
            "schema" => TokenKind::Keyword("schema"),
            "property" => TokenKind::Keyword("property"),
            "struct" => TokenKind::Keyword("struct"),
            "macro" => TokenKind::Keyword("macro"),
            "use" => TokenKind::Keyword("use"),
            "with" => TokenKind::Keyword("with"),
            "as" => TokenKind::Keyword("as"),
            "show" => TokenKind::Keyword("show"),
            "id" => TokenKind::Keyword("id"),
            "inv" => TokenKind::Keyword("inv"),
            "title" => TokenKind::Keyword("title"),
            "author" => TokenKind::Keyword("author"),
            "abstract" => TokenKind::Keyword("abstract"),
            _ => TokenKind::Ident(ident.to_owned()),
        };
        Token {
            kind,
            span: Span::new(start, self.cursor),
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while self.peek().is_some_and(|b| b.is_ascii_whitespace()) {
                self.cursor += 1;
            }
            if self.peek() == Some(b'/') && self.peek_next() == Some(b'/') {
                self.cursor += 2;
                while self.peek().is_some_and(|b| b != b'\n') {
                    self.cursor += 1;
                }
            } else if self.peek() == Some(b'#') {
                while self.peek().is_some_and(|b| b != b'\n') {
                    self.cursor += 1;
                }
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.cursor).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.bytes.get(self.cursor + 1).copied()
    }

    fn peek_offset(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.cursor + offset).copied()
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            cursor: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_program(&mut self) -> Ast {
        let mut statements = Vec::new();
        while !self.at(TokenKind::Eof) {
            if let Some(stmt) = self.parse_stmt() {
                statements.push(stmt);
            } else {
                self.synchronize();
            }
        }
        Ast { statements }
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        match &self.peek().kind {
            TokenKind::Keyword("cell") => self.parse_cell().map(Stmt::Cell),
            TokenKind::Keyword("prove") | TokenKind::Keyword("construct") => {
                self.parse_proof().map(Stmt::Proof)
            }
            TokenKind::Keyword("folder") => self.parse_folder().map(Stmt::Folder),
            TokenKind::Keyword("schema") | TokenKind::Keyword("property") => self
                .parse_declaration(DeclarationKind::Schema)
                .map(Stmt::Declaration),
            TokenKind::Keyword("struct") => self
                .parse_declaration(DeclarationKind::Struct)
                .map(Stmt::Declaration),
            TokenKind::Keyword("macro") => self
                .parse_declaration(DeclarationKind::Macro)
                .map(Stmt::Declaration),
            TokenKind::Keyword("use") => self.parse_use().map(Stmt::Use),
            TokenKind::Keyword("show") => self.parse_show(),
            TokenKind::Keyword("title")
            | TokenKind::Keyword("author")
            | TokenKind::Keyword("abstract") => self.parse_metadata(),
            TokenKind::Eof => None,
            _ => {
                let token = self.bump().clone();
                self.diagnostics
                    .push(Diagnostic::error("expected a statement", token.span));
                None
            }
        }
    }

    fn parse_block_body(&mut self, context: &'static str) -> Option<(Vec<Stmt>, Token)> {
        let open_message = if context == "declaration" {
            "expected `{` before declaration body"
        } else {
            "expected `{` before folder body"
        };
        let close_message = if context == "declaration" {
            "expected `}` after declaration body"
        } else {
            "expected `}` after folder body"
        };
        self.expect(TokenKind::LBrace, open_message)?;
        let mut body = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            if let Some(stmt) = self.parse_stmt() {
                body.push(stmt);
            } else {
                self.synchronize();
            }
        }
        let end = self.expect(TokenKind::RBrace, close_message)?;
        Some((body, end))
    }

    fn parse_metadata(&mut self) -> Option<Stmt> {
        let start = self.bump().clone();
        let key = match start.kind {
            TokenKind::Keyword("title") => MetadataKey::Title,
            TokenKind::Keyword("author") => MetadataKey::Author,
            TokenKind::Keyword("abstract") => MetadataKey::Abstract,
            _ => unreachable!(),
        };
        let value = self.expect_string("expected a string literal")?;
        let end = self.expect(TokenKind::Semicolon, "expected `;` after metadata")?;
        Some(Stmt::Metadata {
            key,
            value,
            span: start.span.join(end.span),
        })
    }

    fn parse_cell(&mut self) -> Option<CellDecl> {
        let start = self.bump().span;
        let (name, _) = self.parse_name()?;
        let (source, target, invertibility) = if self.eat(TokenKind::Colon).is_some() {
            let source = self.parse_expr()?;
            let invertibility = self.parse_relation("cell declaration")?;
            let target = self.parse_expr()?;
            (Some(source), Some(target), invertibility)
        } else {
            (None, None, Invertibility::Directed)
        };
        let end = self.expect(TokenKind::Semicolon, "expected `;` after cell declaration")?;
        Some(CellDecl {
            name,
            source,
            target,
            invertibility,
            span: start.join(end.span),
        })
    }

    fn parse_relation(&mut self, context: &'static str) -> Option<Invertibility> {
        let token = self.bump().clone();
        match token.kind {
            TokenKind::Arrow => Some(Invertibility::Directed),
            TokenKind::DoubleArrow => Some(Invertibility::Invertible),
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    format!("expected `->` or `<->` in {context}"),
                    token.span,
                ));
                None
            }
        }
    }

    fn parse_proof(&mut self) -> Option<ProofDecl> {
        let start = self.bump().span;
        let (name, _) = self.parse_name()?;
        self.expect(TokenKind::Colon, "expected `:` after proof name")?;
        let source = self.parse_expr()?;
        let invertibility = self.parse_relation("proof declaration")?;
        let target = self.parse_expr()?;
        let (steps, end) = if self.eat(TokenKind::LBrace).is_some() {
            let mut steps = Vec::new();
            while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                if let Some(step) = self.parse_proof_step() {
                    steps.push(step);
                } else {
                    self.synchronize();
                }
            }
            let end = self.expect(TokenKind::RBrace, "expected `}` after proof body")?;
            (steps, end)
        } else {
            let end = self.expect(TokenKind::Semicolon, "expected `;` after proof declaration")?;
            (Vec::new(), end)
        };
        Some(ProofDecl {
            name,
            source,
            target,
            invertibility,
            steps,
            span: start.join(end.span),
        })
    }

    fn parse_proof_step(&mut self) -> Option<ProofStep> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Keyword("attach") => {
                let start = self.bump().span;
                let expr = self.parse_expr()?;
                let end = self.expect(TokenKind::Semicolon, "expected `;` after attach step")?;
                Some(ProofStep::Attach {
                    expr,
                    span: start.join(end.span),
                })
            }
            TokenKind::Keyword("contract") => {
                let start = self.bump().span;
                let bias = if self.at(TokenKind::Semicolon) {
                    None
                } else {
                    Some(self.parse_contraction_bias()?)
                };
                let end = self.expect(TokenKind::Semicolon, "expected `;` after contract step")?;
                Some(ProofStep::Contract {
                    bias,
                    span: start.join(end.span),
                })
            }
            _ => {
                self.diagnostics
                    .push(Diagnostic::error("expected proof step", token.span));
                None
            }
        }
    }

    fn parse_contraction_bias(&mut self) -> Option<ContractionBias> {
        let token = self.bump().clone();
        let TokenKind::Ident(name) = token.kind else {
            self.diagnostics.push(Diagnostic::error(
                "expected contraction bias `lower`, `higher`, or `same`",
                token.span,
            ));
            return None;
        };
        match name.as_str() {
            "lower" => Some(ContractionBias::Lower),
            "higher" => Some(ContractionBias::Higher),
            "same" => Some(ContractionBias::Same),
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    "expected contraction bias `lower`, `higher`, or `same`",
                    token.span,
                ));
                None
            }
        }
    }

    fn parse_folder(&mut self) -> Option<FolderDecl> {
        let start = self.bump().span;
        let (name, _) = self.parse_name()?;
        let (body, end) = self.parse_block_body("folder")?;
        Some(FolderDecl {
            name,
            body,
            span: start.join(end.span),
        })
    }

    fn parse_declaration(&mut self, kind: DeclarationKind) -> Option<DeclarationDecl> {
        let start = self.bump().span;
        let (name, _) = self.parse_name()?;
        self.expect(TokenKind::LParen, "expected `(` after declaration name")?;
        let mut params = Vec::new();
        if !self.at(TokenKind::RParen) {
            loop {
                let (param_name, param_span) = self.parse_name()?;
                self.expect(TokenKind::Colon, "expected `:` after parameter name")?;
                let ty = self.parse_param_type()?;
                params.push(Param {
                    name: param_name,
                    ty,
                    span: param_span,
                });
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "expected `)` after parameters")?;
        let (body, end) = self.parse_block_body("declaration")?;
        Some(DeclarationDecl {
            kind,
            name,
            params,
            body,
            span: start.join(end.span),
        })
    }

    fn parse_param_type(&mut self) -> Option<ParamType> {
        if self.at_keyword("cell") {
            self.bump();
            self.expect(TokenKind::Lt, "expected `<` in parameter type")?;
            let dimension = self.expect_number("expected parameter dimension")?;
            self.expect(TokenKind::Gt, "expected `>` in parameter type")?;
            return Some(ParamType::Cell(dimension));
        }

        let (name, _) = self.parse_name()?;
        Some(ParamType::Structure(name))
    }

    fn parse_use(&mut self) -> Option<UseDecl> {
        let start = self.bump().span;
        let (declaration, _) = self.parse_name()?;
        self.expect(TokenKind::LParen, "expected `(` after declaration name")?;
        let mut args = Vec::new();
        if !self.at(TokenKind::RParen) {
            loop {
                let (arg, _) = self.parse_name()?;
                args.push(arg);
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(
            TokenKind::RParen,
            "expected `)` after declaration arguments",
        )?;
        self.expect_keyword("as", "expected `as` before declaration alias")?;
        let (alias, _) = self.parse_name()?;
        let (bindings, end) = if self.at_keyword("with") {
            self.bump();
            self.expect(TokenKind::LBrace, "expected `{` after `with`")?;
            let mut bindings = Vec::new();
            while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                let (field, field_span) = self.parse_name()?;
                self.expect(TokenKind::Equals, "expected `=` in `with` binding")?;
                let expr = self.parse_expr()?;
                let end = self.expect(TokenKind::Semicolon, "expected `;` after `with` binding")?;
                bindings.push(UseBinding {
                    field,
                    expr,
                    span: field_span.join(end.span),
                });
            }
            let end = self.expect(TokenKind::RBrace, "expected `}` after `with` block")?;
            if self.at(TokenKind::Semicolon) {
                let semicolon = self.bump().clone();
                (bindings, semicolon)
            } else {
                (bindings, end)
            }
        } else {
            let end = self.expect(TokenKind::Semicolon, "expected `;` after declaration use")?;
            (Vec::new(), end)
        };
        Some(UseDecl {
            declaration,
            args,
            alias,
            bindings,
            span: start.join(end.span),
        })
    }

    fn parse_show(&mut self) -> Option<Stmt> {
        let start = self.bump().span;
        let expr = self.parse_expr()?;
        let end = self.expect(TokenKind::Semicolon, "expected `;` after show statement")?;
        Some(Stmt::Show {
            expr,
            span: start.join(end.span),
        })
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        let mut terms = vec![self.parse_term()?];
        while self.eat(TokenKind::Star).is_some() {
            terms.push(self.parse_term()?);
        }

        if terms.len() == 1 {
            terms.pop()
        } else {
            let span = terms
                .first()
                .unwrap()
                .span()
                .join(terms.last().unwrap().span());
            Some(Expr::Compose { terms, span })
        }
    }

    fn parse_term(&mut self) -> Option<Expr> {
        if self.at_keyword("id") {
            let start = self.bump().span;
            self.expect(TokenKind::LParen, "expected `(` after id")?;
            let expr = self.parse_expr()?;
            let end = self.expect(TokenKind::RParen, "expected `)` after identity expression")?;
            return Some(Expr::Identity {
                expr: Box::new(expr),
                span: start.join(end.span),
            });
        }
        if self.at_keyword("inv") {
            let start = self.bump().span;
            self.expect(TokenKind::LParen, "expected `(` after inv")?;
            let expr = self.parse_expr()?;
            let end = self.expect(TokenKind::RParen, "expected `)` after inverse expression")?;
            return Some(Expr::Inverse {
                expr: Box::new(expr),
                span: start.join(end.span),
            });
        }
        if self.at_keyword("contract") {
            let start = self.bump().span;
            self.expect(TokenKind::LParen, "expected `(` after contract")?;
            let expr = self.parse_expr()?;
            let bias = if self.eat(TokenKind::Comma).is_some() {
                Some(self.parse_contraction_bias()?)
            } else {
                None
            };
            let end = self.expect(TokenKind::RParen, "expected `)` after contract expression")?;
            return Some(Expr::Contract {
                expr: Box::new(expr),
                bias,
                span: start.join(end.span),
            });
        }
        if self.eat(TokenKind::LParen).is_some() {
            let expr = self.parse_expr()?;
            self.expect(TokenKind::RParen, "expected `)` after expression")?;
            return Some(expr);
        }
        let (name, span) = self.parse_name()?;
        Some(Expr::Name { name, span })
    }

    fn parse_name(&mut self) -> Option<(String, Span)> {
        let token = self.bump().clone();
        let TokenKind::Ident(first) = token.kind else {
            self.diagnostics
                .push(Diagnostic::error("expected a name", token.span));
            return None;
        };

        let mut name = first;
        let mut span = token.span;
        while self.eat(TokenKind::Dot).is_some() {
            let token = self.bump().clone();
            let TokenKind::Ident(segment) = token.kind else {
                self.diagnostics.push(Diagnostic::error(
                    "expected a name segment after `.`",
                    token.span,
                ));
                return None;
            };
            name.push('.');
            name.push_str(&segment);
            span = span.join(token.span);
        }
        Some((name, span))
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> Option<Token> {
        if self.at(kind.clone()) {
            Some(self.bump().clone())
        } else {
            let span = self.peek().span;
            self.diagnostics.push(Diagnostic::error(message, span));
            None
        }
    }

    fn expect_keyword(&mut self, keyword: &'static str, message: &str) -> Option<Token> {
        if self.at_keyword(keyword) {
            Some(self.bump().clone())
        } else {
            let span = self.peek().span;
            self.diagnostics.push(Diagnostic::error(message, span));
            None
        }
    }

    fn expect_string(&mut self, message: &str) -> Option<String> {
        let token = self.bump().clone();
        if let TokenKind::String(value) = token.kind {
            Some(value)
        } else {
            self.diagnostics
                .push(Diagnostic::error(message, token.span));
            None
        }
    }

    fn expect_number(&mut self, message: &str) -> Option<usize> {
        let token = self.bump().clone();
        if let TokenKind::Number(value) = token.kind {
            Some(value)
        } else {
            self.diagnostics
                .push(Diagnostic::error(message, token.span));
            None
        }
    }

    fn synchronize(&mut self) {
        while !self.at(TokenKind::Eof) {
            if self.eat(TokenKind::Semicolon).is_some() || self.at(TokenKind::RBrace) {
                return;
            }
            self.cursor += 1;
        }
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        self.at(kind).then(|| self.bump().clone())
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn at_keyword(&self, keyword: &'static str) -> bool {
        self.peek().kind == TokenKind::Keyword(keyword)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn bump(&mut self) -> &Token {
        let cursor = self.cursor;
        self.cursor = (self.cursor + 1).min(self.tokens.len() - 1);
        &self.tokens[cursor]
    }
}

#[derive(Default)]
struct Scope {
    aliases: HashMap<String, String>,
    instance_aliases: HashMap<String, String>,
    provided: HashMap<String, ProvidedBinding>,
    prefix: Option<String>,
    folder: Option<Node>,
}

#[derive(Debug, Clone)]
struct ProvidedBinding {
    target: String,
    span: Span,
}

#[derive(Debug, Clone)]
struct InstanceInfo {
    declaration: String,
    canonical_prefix: String,
    aliases: BTreeMap<String, String>,
}

impl DeclarationKind {
    fn noun(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::Struct => "struct",
            Self::Macro => "macro",
        }
    }

    fn is_applicative(self) -> bool {
        matches!(self, Self::Schema)
    }
}

struct Compiler {
    signature: Signature,
    symbols: BTreeMap<String, Symbol>,
    declarations: HashMap<String, DeclarationDecl>,
    instances: HashMap<String, InstanceInfo>,
    applicative_instances: HashMap<String, InstanceInfo>,
    diagnostics: Vec<Diagnostic>,
    selected: Option<String>,
    selected_diagram: Option<Diagram>,
    metadata: Metadata,
    next_generator_id: usize,
    expansion_stack: Vec<String>,
}

impl Compiler {
    fn new(options: CompileOptions) -> Self {
        let metadata = Metadata {
            title: options.title,
            author: options.author,
            abstr: options.abstr,
        };
        Self {
            signature: Signature::default(),
            symbols: BTreeMap::new(),
            declarations: HashMap::new(),
            instances: HashMap::new(),
            applicative_instances: HashMap::new(),
            diagnostics: Vec::new(),
            selected: None,
            selected_diagram: None,
            metadata,
            next_generator_id: 0,
            expansion_stack: Vec::new(),
        }
    }

    fn compile_ast(&mut self, ast: &Ast) {
        let mut seen_declarations = HashSet::new();
        self.collect_declarations(&ast.statements, &mut seen_declarations);

        let mut scope = Scope::default();
        for stmt in &ast.statements {
            if matches!(stmt, Stmt::Declaration(_)) {
                continue;
            }
            self.compile_stmt(stmt, &mut scope);
        }
    }

    fn collect_declarations(
        &mut self,
        statements: &[Stmt],
        seen_declarations: &mut HashSet<String>,
    ) {
        for stmt in statements {
            match stmt {
                Stmt::Declaration(schema) => {
                    if !seen_declarations.insert(schema.name.clone()) {
                        self.error(
                            format!("duplicate declaration `{}`", schema.name),
                            schema.span,
                        );
                    } else {
                        self.declarations
                            .insert(schema.name.clone(), schema.clone());
                    }
                }
                Stmt::Folder(folder) => self.collect_declarations(&folder.body, seen_declarations),
                _ => {}
            }
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt, scope: &mut Scope) {
        match stmt {
            Stmt::Metadata { key, value, .. } => match key {
                MetadataKey::Title => self.metadata.title = Some(value.clone()),
                MetadataKey::Author => self.metadata.author = Some(value.clone()),
                MetadataKey::Abstract => self.metadata.abstr = Some(value.clone()),
            },
            Stmt::Cell(decl) => self.compile_cell(decl, scope),
            Stmt::Proof(decl) => self.compile_proof(decl, scope),
            Stmt::Folder(decl) => self.compile_folder(decl, scope),
            Stmt::Use(decl) => self.compile_use(decl, scope),
            Stmt::Show { expr, span } => {
                let Some(diagram) = self.compile_expr(expr, scope) else {
                    return;
                };
                if let Err(error) = diagram.check(true) {
                    self.error(format!("shown diagram failed validation: {error:?}"), *span);
                    return;
                }
                self.selected = Some(self.format_expr(expr, scope));
                self.selected_diagram = Some(diagram);
            }
            Stmt::Declaration(decl) => {
                self.error(
                    format!(
                        "nested {} `{}` is not supported in V1",
                        decl.kind.noun(),
                        decl.name
                    ),
                    decl.span,
                );
            }
        }
    }

    fn compile_folder(&mut self, decl: &FolderDecl, scope: &mut Scope) {
        let parent = scope.folder.unwrap_or_else(|| self.signature.root_node());
        let Some(folder) = self.signature.create_folder(parent, decl.name.clone()) else {
            self.error("could not create signature folder", decl.span);
            return;
        };

        let previous_folder = scope.folder.replace(folder);
        for stmt in &decl.body {
            if matches!(stmt, Stmt::Declaration(_)) {
                continue;
            }
            self.compile_stmt(stmt, scope);
        }
        scope.folder = previous_folder;
    }

    fn compile_cell(&mut self, decl: &CellDecl, scope: &mut Scope) {
        let public_name = self.declared_name(&decl.name, scope);
        if let Some(provided) = scope.provided.get(&decl.name).cloned() {
            self.bind_provided_cell(decl, &public_name, &provided, scope);
            return;
        }

        if self.symbols.contains_key(&public_name) {
            self.error(format!("duplicate symbol `{public_name}`"), decl.span);
            return;
        }

        let diagram: Diagram = match (&decl.source, &decl.target) {
            (None, None) => {
                let generator = self.next_generator(0);
                Diagram0::new(generator, Orientation::Positive).into()
            }
            (Some(source), Some(target)) => {
                let Some(source) = self.compile_expr(source, scope) else {
                    return;
                };
                let Some(target) = self.compile_expr(target, scope) else {
                    return;
                };
                if source.dimension() != target.dimension() {
                    self.error(
                        format!(
                            "source dimension {} does not match target dimension {}",
                            source.dimension(),
                            target.dimension()
                        ),
                        decl.span,
                    );
                    return;
                }

                let generator = self.next_generator(source.dimension() + 1);
                match DiagramN::from_generator(generator, source.clone(), target.clone()) {
                    Ok(diagram) => diagram.into(),
                    Err(error) => {
                        self.error(format!("could not create generator: {error:?}"), decl.span);
                        return;
                    }
                }
            }
            _ => {
                self.error(
                    "cell declaration must have both source and target",
                    decl.span,
                );
                return;
            }
        };

        let generator = diagram.max_generator().generator;
        if let Err(error) = diagram.check(true) {
            self.error(
                format!("compiled diagram failed validation: {error:?}"),
                decl.span,
            );
            return;
        }

        let info = GeneratorInfo {
            generator,
            oriented: false,
            invertibility: decl.invertibility,
            single_preview: true,
            color: Color::from_str(COLORS[generator.id % COLORS.len()]).unwrap(),
            shape: VertexShape::default(),
            diagram: diagram.clone(),
            name: public_name.clone(),
        };
        let parent = scope.folder.unwrap_or_else(|| self.signature.root_node());
        if self
            .signature
            .insert_item_onto(parent, SignatureItem::Item(info))
            .is_none()
        {
            self.error("could not insert cell into signature folder", decl.span);
            return;
        }

        if scope.prefix.is_some() {
            scope.aliases.insert(decl.name.clone(), public_name.clone());
        }
        self.insert_symbol(public_name, diagram, decl.invertibility);
    }

    fn compile_proof(&mut self, decl: &ProofDecl, scope: &mut Scope) {
        let public_name = self.declared_name(&decl.name, scope);
        if let Some(provided) = scope.provided.get(&decl.name).cloned() {
            self.bind_provided_proof(decl, &public_name, &provided, scope);
            return;
        }

        if self.symbols.contains_key(&public_name) {
            self.error(format!("duplicate symbol `{public_name}`"), decl.span);
            return;
        }

        let Some(source) = self.compile_expr(&decl.source, scope) else {
            return;
        };
        let Some(target) = self.compile_expr(&decl.target, scope) else {
            return;
        };
        let diagram = if decl.steps.is_empty() {
            self.construct_proof(&source, &target, decl.span)
        } else {
            self.construct_scripted_proof(&source, &target, &decl.steps, scope, decl.span)
        };
        let Some(diagram) = diagram else {
            return;
        };
        if let Err(error) = diagram.check(true) {
            self.error(
                format!("constructed proof failed validation: {error:?}"),
                decl.span,
            );
            return;
        }

        if scope.prefix.is_some() {
            scope.aliases.insert(decl.name.clone(), public_name.clone());
        }
        self.insert_symbol(public_name, diagram, decl.invertibility);
    }

    fn bind_provided_cell(
        &mut self,
        decl: &CellDecl,
        public_name: &str,
        provided: &ProvidedBinding,
        scope: &mut Scope,
    ) {
        let Some(symbol) = self.symbols.get(&provided.target).cloned() else {
            self.error(
                format!("unknown provided symbol `{}`", provided.target),
                provided.span,
            );
            return;
        };

        if decl.invertibility.is_invertible() && !symbol.invertibility.is_invertible() {
            self.error(
                format!(
                    "provided symbol `{}` is directed but `{}` expects an invertible cell",
                    provided.target, decl.name
                ),
                provided.span,
            );
            return;
        }

        match (&decl.source, &decl.target) {
            (None, None) => {
                if symbol.diagram.dimension() != 0 {
                    self.error(
                        format!(
                            "provided symbol `{}` has dimension {}, expected 0",
                            provided.target,
                            symbol.diagram.dimension()
                        ),
                        provided.span,
                    );
                    return;
                }
            }
            (Some(source), Some(target)) => {
                let Some(source) = self.compile_expr(source, scope) else {
                    return;
                };
                let Some(target) = self.compile_expr(target, scope) else {
                    return;
                };
                if source.dimension() != target.dimension() {
                    self.error(
                        format!(
                            "source dimension {} does not match target dimension {}",
                            source.dimension(),
                            target.dimension()
                        ),
                        decl.span,
                    );
                    return;
                }
                if symbol.diagram.dimension() != source.dimension() + 1 {
                    self.error(
                        format!(
                            "provided symbol `{}` has dimension {}, expected {}",
                            provided.target,
                            symbol.diagram.dimension(),
                            source.dimension() + 1
                        ),
                        provided.span,
                    );
                    return;
                }
                let Ok(diagram) = DiagramN::try_from(symbol.diagram.clone()) else {
                    self.error(
                        format!(
                            "provided symbol `{}` has no source/target boundary",
                            provided.target
                        ),
                        provided.span,
                    );
                    return;
                };
                if diagram.source() != source || diagram.target() != target {
                    self.error(
                        format!(
                            "provided symbol `{}` does not match the declared boundary of `{}`",
                            provided.target, decl.name
                        ),
                        provided.span,
                    );
                    return;
                }
            }
            _ => {
                self.error(
                    "cell declaration must have both source and target",
                    decl.span,
                );
                return;
            }
        }

        scope
            .aliases
            .insert(decl.name.clone(), provided.target.clone());
        if scope.prefix.is_some() {
            scope
                .aliases
                .insert(public_name.to_owned(), provided.target.clone());
        }
    }

    fn bind_provided_proof(
        &mut self,
        decl: &ProofDecl,
        public_name: &str,
        provided: &ProvidedBinding,
        scope: &mut Scope,
    ) {
        let Some(source) = self.compile_expr(&decl.source, scope) else {
            return;
        };
        let Some(target) = self.compile_expr(&decl.target, scope) else {
            return;
        };
        if source.dimension() != target.dimension() {
            self.error(
                format!(
                    "proof source dimension {} does not match target dimension {}",
                    source.dimension(),
                    target.dimension()
                ),
                decl.span,
            );
            return;
        }

        let Some(symbol) = self.symbols.get(&provided.target).cloned() else {
            self.error(
                format!("unknown provided symbol `{}`", provided.target),
                provided.span,
            );
            return;
        };
        if decl.invertibility.is_invertible() && !symbol.invertibility.is_invertible() {
            self.error(
                format!(
                    "provided symbol `{}` is directed but `{}` expects an invertible proof",
                    provided.target, decl.name
                ),
                provided.span,
            );
            return;
        }
        if symbol.diagram.dimension() != source.dimension() + 1 {
            self.error(
                format!(
                    "provided symbol `{}` has dimension {}, expected {}",
                    provided.target,
                    symbol.diagram.dimension(),
                    source.dimension() + 1
                ),
                provided.span,
            );
            return;
        }
        let Ok(diagram) = DiagramN::try_from(symbol.diagram.clone()) else {
            self.error(
                format!(
                    "provided symbol `{}` has no source/target boundary",
                    provided.target
                ),
                provided.span,
            );
            return;
        };
        if diagram.source() != source || diagram.target() != target {
            self.error(
                format!(
                    "provided symbol `{}` does not match the declared boundary of `{}`",
                    provided.target, decl.name
                ),
                provided.span,
            );
            return;
        }

        scope
            .aliases
            .insert(decl.name.clone(), provided.target.clone());
        if scope.prefix.is_some() {
            scope
                .aliases
                .insert(public_name.to_owned(), provided.target.clone());
        }
    }

    fn construct_scripted_proof(
        &mut self,
        source: &Diagram,
        target: &Diagram,
        steps: &[ProofStep],
        scope: &Scope,
        span: Span,
    ) -> Option<Diagram> {
        if source.dimension() != target.dimension() {
            self.error(
                format!(
                    "proof source dimension {} does not match target dimension {}",
                    source.dimension(),
                    target.dimension()
                ),
                span,
            );
            return None;
        }

        let Ok(source_n) = DiagramN::try_from(source.clone()) else {
            self.error("only positive-dimensional proofs can be constructed", span);
            return None;
        };
        let mut proof = source_n.identity();

        for step in steps {
            match step {
                ProofStep::Attach { expr, span } => {
                    let Some(diagram) = self.compile_expr(expr, scope) else {
                        return None;
                    };
                    let Ok(candidate) = DiagramN::try_from(diagram) else {
                        self.error(
                            "attach step must name a positive-dimensional diagram",
                            *span,
                        );
                        return None;
                    };
                    if candidate.dimension() != proof.dimension() {
                        self.error(
                            format!(
                                "attach step has dimension {}, expected {}",
                                candidate.dimension(),
                                proof.dimension()
                            ),
                            *span,
                        );
                        return None;
                    }

                    let endpoint = proof.target();
                    let mut embeddings = endpoint.embeddings(&candidate.source());
                    let Some(embedding) = embeddings.next() else {
                        self.error("attach source does not embed in the proof target", *span);
                        return None;
                    };
                    match proof.attach(&candidate, Boundary::Target, &embedding) {
                        Ok(next) => proof = next,
                        Err(error) => {
                            self.error(format!("attach step failed: {error:?}"), *span);
                            return None;
                        }
                    }
                }
                ProofStep::Contract { bias, span } => {
                    let Some(next) = self.contract_target(&proof, *bias, *span) else {
                        return None;
                    };
                    proof = next;
                }
            }
        }

        if proof.target() != *target {
            self.error("proof body did not construct the requested target", span);
            return None;
        }

        Some(proof.into())
    }

    fn compile_use(&mut self, decl: &UseDecl, scope: &mut Scope) {
        let declaration_name = self.resolve_name(&decl.declaration, scope);
        let Some(schema) = self.declarations.get(&declaration_name).cloned() else {
            self.error(
                format!("unknown declaration `{}`", decl.declaration),
                decl.span,
            );
            return;
        };

        if self.expansion_stack.contains(&schema.name) {
            self.error(
                format!(
                    "recursive declaration expansion involving `{}`",
                    schema.name
                ),
                decl.span,
            );
            return;
        }

        if schema.params.len() != decl.args.len() {
            self.error(
                format!(
                    "{} `{}` expects {} arguments but received {}",
                    schema.kind.noun(),
                    schema.name,
                    schema.params.len(),
                    decl.args.len()
                ),
                decl.span,
            );
            return;
        }

        let public_alias = self.declared_name(&decl.alias, scope);
        if self.instances.contains_key(&public_alias) {
            self.error(format!("duplicate instance `{public_alias}`"), decl.span);
            return;
        }

        let mut resolved_args = Vec::new();
        let mut cell_param_aliases = Vec::new();
        let mut structure_param_aliases = Vec::new();
        let mut child = Scope {
            aliases: HashMap::new(),
            instance_aliases: HashMap::new(),
            provided: HashMap::new(),
            prefix: Some(public_alias.clone()),
            folder: scope.folder,
        };

        for (param, arg) in schema.params.iter().zip(&decl.args) {
            match &param.ty {
                ParamType::Cell(dimension) => {
                    let resolved_arg = self.resolve_name(arg, scope);
                    let Some(symbol) = self.symbols.get(&resolved_arg) else {
                        self.error(format!("unknown argument `{arg}`"), param.span);
                        return;
                    };
                    if symbol.info.dimension != *dimension {
                        self.error(
                            format!(
                                "argument `{arg}` has dimension {}, expected {}",
                                symbol.info.dimension, dimension
                            ),
                            param.span,
                        );
                        return;
                    }
                    resolved_args.push(resolved_arg.clone());
                    cell_param_aliases.push((param.name.clone(), resolved_arg));
                }
                ParamType::Structure(expected) => {
                    let resolved_arg = self.resolve_instance_name(arg, scope);
                    let Some(instance) = self.instances.get(&resolved_arg).cloned() else {
                        self.error(format!("unknown structure argument `{arg}`"), param.span);
                        return;
                    };
                    if instance.declaration != *expected {
                        self.error(
                            format!(
                                "structure argument `{arg}` has type `{}`, expected `{expected}`",
                                instance.declaration
                            ),
                            param.span,
                        );
                        return;
                    }
                    resolved_args.push(instance.canonical_prefix.clone());
                    structure_param_aliases.push((param.name.clone(), resolved_arg, instance));
                }
            }
        }

        let instance_key = format!("{}({})", schema.name, resolved_args.join(","));
        if schema.kind.is_applicative() {
            if let Some(instance) = self.applicative_instances.get(&instance_key).cloned() {
                if !decl.bindings.is_empty() {
                    let Some(provided) = self.compile_provided_bindings(decl, &schema, scope)
                    else {
                        return;
                    };
                    if !self.validate_existing_instance_bindings(&instance, &provided) {
                        return;
                    }
                }
                self.alias_existing_instance(&public_alias, &instance, scope);
                return;
            }
        }

        if let Some(provided) = self.compile_provided_bindings(decl, &schema, scope) {
            child.provided = provided;
        } else {
            return;
        }

        for (name, target) in cell_param_aliases {
            child.aliases.insert(name.clone(), target.clone());
            child
                .aliases
                .insert(format!("{public_alias}.{name}"), target);
        }
        for (name, resolved_instance, instance) in structure_param_aliases {
            child
                .instance_aliases
                .insert(name.clone(), resolved_instance.clone());
            for (field, target) in instance.aliases {
                child
                    .aliases
                    .insert(format!("{name}.{field}"), target.clone());
                child
                    .aliases
                    .insert(format!("{public_alias}.{name}.{field}"), target);
            }
        }

        self.expansion_stack.push(schema.name.clone());
        for stmt in &schema.body {
            self.compile_stmt(stmt, &mut child);
        }
        self.expansion_stack.pop();

        self.propagate_public_aliases(&public_alias, &child, scope);
        let instance = self.collect_instance_info(&schema, &public_alias, &child);
        self.instances
            .insert(public_alias.clone(), instance.clone());
        if schema.kind.is_applicative() {
            self.applicative_instances.insert(instance_key, instance);
        }
    }

    fn compile_provided_bindings(
        &mut self,
        decl: &UseDecl,
        declaration: &DeclarationDecl,
        scope: &Scope,
    ) -> Option<HashMap<String, ProvidedBinding>> {
        let direct_fields = Self::direct_field_names(&declaration.body);
        let mut bindings = HashMap::new();
        for binding in &decl.bindings {
            if !direct_fields.contains(&binding.field) {
                self.error(
                    format!(
                        "{} `{}` has no directly fillable field `{}`",
                        declaration.kind.noun(),
                        declaration.name,
                        binding.field
                    ),
                    binding.span,
                );
                return None;
            }
            if bindings.contains_key(&binding.field) {
                self.error(
                    format!("duplicate `with` binding for `{}`", binding.field),
                    binding.span,
                );
                return None;
            }
            let Some(target) = self.binding_target(&binding.expr, scope) else {
                return None;
            };
            bindings.insert(
                binding.field.clone(),
                ProvidedBinding {
                    target,
                    span: binding.span,
                },
            );
        }
        Some(bindings)
    }

    fn direct_field_names(body: &[Stmt]) -> HashSet<String> {
        body.iter()
            .filter_map(|stmt| match stmt {
                Stmt::Cell(decl) => Some(decl.name.clone()),
                Stmt::Proof(decl) => Some(decl.name.clone()),
                _ => None,
            })
            .collect()
    }

    fn binding_target(&mut self, expr: &Expr, scope: &Scope) -> Option<String> {
        let Expr::Name { name, span } = expr else {
            self.error(
                "`with` bindings must name an existing symbol in V1",
                expr.span(),
            );
            return None;
        };
        let target = self.resolve_name(name, scope);
        if !self.symbols.contains_key(&target) {
            self.error(format!("unknown provided symbol `{name}`"), *span);
            return None;
        }
        Some(target)
    }

    fn alias_existing_instance(
        &mut self,
        public_alias: &str,
        instance: &InstanceInfo,
        scope: &mut Scope,
    ) {
        for (field, target) in &instance.aliases {
            scope
                .aliases
                .insert(format!("{public_alias}.{field}"), target.clone());
        }
        self.instances.insert(
            public_alias.to_owned(),
            InstanceInfo {
                declaration: instance.declaration.clone(),
                canonical_prefix: instance.canonical_prefix.clone(),
                aliases: instance.aliases.clone(),
            },
        );
    }

    fn validate_existing_instance_bindings(
        &mut self,
        instance: &InstanceInfo,
        provided: &HashMap<String, ProvidedBinding>,
    ) -> bool {
        for (field, binding) in provided {
            let Some(existing) = instance.aliases.get(field) else {
                self.error(
                    format!(
                        "canonical instance `{}` has no field `{field}`",
                        instance.canonical_prefix
                    ),
                    binding.span,
                );
                return false;
            };
            let existing_symbol = self.symbols.get(existing);
            let provided_symbol = self.symbols.get(&binding.target);
            let matches = match (existing_symbol, provided_symbol) {
                (Some(existing), Some(provided)) => existing.diagram == provided.diagram,
                _ => existing == &binding.target,
            };
            if !matches {
                self.error(
                    format!(
                        "provided field `{field}` does not match the canonical `{}` instance",
                        instance.declaration
                    ),
                    binding.span,
                );
                return false;
            }
        }
        true
    }

    fn propagate_public_aliases(&self, public_alias: &str, child: &Scope, parent: &mut Scope) {
        let prefix = format!("{public_alias}.");
        for (name, target) in &child.aliases {
            if name.starts_with(&prefix) {
                parent.aliases.insert(name.clone(), target.clone());
            }
        }
    }

    fn collect_instance_info(
        &self,
        declaration: &DeclarationDecl,
        public_alias: &str,
        child: &Scope,
    ) -> InstanceInfo {
        let prefix = format!("{public_alias}.");
        let mut aliases = BTreeMap::new();
        for name in self.symbols.keys() {
            if let Some(field) = name.strip_prefix(&prefix) {
                aliases.insert(field.to_owned(), name.clone());
            }
        }
        for (name, target) in &child.aliases {
            if let Some(field) = name.strip_prefix(&prefix) {
                aliases.insert(field.to_owned(), target.clone());
            }
        }
        InstanceInfo {
            declaration: declaration.name.clone(),
            canonical_prefix: public_alias.to_owned(),
            aliases,
        }
    }

    fn construct_proof(
        &mut self,
        source: &Diagram,
        target: &Diagram,
        span: Span,
    ) -> Option<Diagram> {
        if source.dimension() != target.dimension() {
            self.error(
                format!(
                    "proof source dimension {} does not match target dimension {}",
                    source.dimension(),
                    target.dimension()
                ),
                span,
            );
            return None;
        }

        let Ok(source) = DiagramN::try_from(source.clone()) else {
            self.error("only positive-dimensional proofs can be constructed", span);
            return None;
        };
        let source_diagram: Diagram = source.clone().into();
        let source_equals_target = source_diagram == *target;

        let mut queue = VecDeque::from([(source.clone().identity(), 0)]);
        let mut visited = HashSet::from([Diagram::from(source.clone().identity())]);
        let candidates = self.proof_candidates(source.dimension() + 1);
        let mut explored = 0usize;

        while let Some((proof, depth)) = queue.pop_front() {
            explored += 1;
            if explored > MAX_PROOF_SEARCH_NODES {
                break;
            }
            if depth >= MAX_PROOF_SEARCH_DEPTH {
                continue;
            }

            for next in self.proof_attachments(&proof, &candidates) {
                if next.source() == source_diagram && next.target() == *target {
                    return Some(next.into());
                }
                let key = Diagram::from(next.clone());
                if visited.insert(key) {
                    queue.push_back((next, depth + 1));
                }
            }

            for next in self.proof_contractions(&proof) {
                if next.source() == source_diagram && next.target() == *target {
                    return Some(next.into());
                }
                let key = Diagram::from(next.clone());
                if visited.insert(key) {
                    queue.push_back((next, depth + 1));
                }
            }
        }

        if source_equals_target {
            return Some(source.identity().into());
        }

        self.error(
            "could not construct proof from existing cells and built-in contraction rules",
            span,
        );
        None
    }

    fn proof_candidates(&self, dimension: usize) -> Vec<DiagramN> {
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();

        for info in self.signature.iter() {
            if info.diagram.dimension() != dimension {
                continue;
            }
            let Ok(diagram) = DiagramN::try_from(info.diagram.clone()) else {
                continue;
            };
            Self::push_proof_candidate(&mut candidates, &mut seen, diagram.clone());
            if info.invertibility.is_invertible() {
                Self::push_proof_candidate(&mut candidates, &mut seen, diagram.inverse());
            }
        }

        for symbol in self.symbols.values() {
            if symbol.diagram.dimension() != dimension {
                continue;
            }
            let Ok(diagram) = DiagramN::try_from(symbol.diagram.clone()) else {
                continue;
            };
            Self::push_proof_candidate(&mut candidates, &mut seen, diagram.clone());
            if symbol.invertibility.is_invertible() {
                Self::push_proof_candidate(&mut candidates, &mut seen, diagram.inverse());
            }
        }

        candidates
    }

    fn push_proof_candidate(
        candidates: &mut Vec<DiagramN>,
        seen: &mut HashSet<Diagram>,
        diagram: DiagramN,
    ) {
        if seen.insert(diagram.clone().into()) {
            candidates.push(diagram);
        }
    }

    fn proof_attachments(&self, proof: &DiagramN, candidates: &[DiagramN]) -> Vec<DiagramN> {
        let endpoint = proof.target();
        candidates
            .iter()
            .flat_map(|candidate| {
                endpoint
                    .embeddings(&candidate.source())
                    .filter_map(|embedding| {
                        proof.attach(candidate, Boundary::Target, &embedding).ok()
                    })
            })
            .collect()
    }

    fn proof_contractions(&self, proof: &DiagramN) -> Vec<DiagramN> {
        let mut contractions = Vec::new();

        for boundary in [Boundary::Target, Boundary::Source] {
            let boundary_size = match boundary {
                Boundary::Source => proof.source(),
                Boundary::Target => proof.target(),
            }
            .size()
            .unwrap_or(0);
            for direction in [Direction::Forward, Direction::Backward] {
                for step in 0..boundary_size {
                    let mut path = [];
                    let Ok(candidate) = proof.clone().contract(
                        boundary.into(),
                        &mut path,
                        0,
                        direction,
                        step,
                        None,
                        &self.signature,
                    ) else {
                        continue;
                    };
                    contractions.push(candidate);
                }
            }
        }

        contractions
    }

    fn contract_target(
        &mut self,
        proof: &DiagramN,
        bias: Option<ContractionBias>,
        span: Span,
    ) -> Option<DiagramN> {
        let Some(boundary_size) = proof.target().size() else {
            self.error("contract step needs a positive-dimensional target", span);
            return None;
        };
        let Some(max_step) = boundary_size.checked_sub(1) else {
            self.error("contract step needs a non-empty target boundary", span);
            return None;
        };
        let step = 1.min(max_step);
        let mut path = [];
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            proof.clone().contract(
                Boundary::Target.into(),
                &mut path,
                0,
                Direction::Forward,
                step,
                bias.map(Into::into),
                &self.signature,
            )
        }));
        match result {
            Ok(Ok(contracted)) => Some(contracted),
            Ok(Err(error)) => {
                self.error(format!("contract step failed: {error:?}"), span);
                None
            }
            Err(_) => {
                self.error(
                    "contract step failed inside the core contraction engine",
                    span,
                );
                None
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr, scope: &Scope) -> Option<Diagram> {
        match expr {
            Expr::Name { name, span } => {
                let resolved = self.resolve_name(name, scope);
                let Some(symbol) = self.symbols.get(&resolved) else {
                    self.error(format!("unknown symbol `{name}`"), *span);
                    return None;
                };
                Some(symbol.diagram.clone())
            }
            Expr::Identity { expr, .. } => {
                let diagram = self.compile_expr(expr, scope)?;
                Some(diagram.identity().into())
            }
            Expr::Inverse { expr, span } => self.compile_inverse(expr, *span, scope),
            Expr::Contract { expr, bias, span } => {
                let diagram = self.compile_expr(expr, scope)?;
                let Ok(diagram) = DiagramN::try_from(diagram) else {
                    self.error(
                        "only positive-dimensional diagrams can be contracted",
                        *span,
                    );
                    return None;
                };
                let proof = diagram.identity();
                Some(self.contract_target(&proof, *bias, *span)?.target())
            }
            Expr::Compose { terms, span } => self.compile_composition(terms, *span, scope),
        }
    }

    fn compile_inverse(&mut self, expr: &Expr, span: Span, scope: &Scope) -> Option<Diagram> {
        let diagram = self.compile_expr(expr, scope)?;
        let invertibility = self
            .expression_invertibility(expr, scope)
            .unwrap_or_else(|| diagram.invertibility(&self.signature));
        if !invertibility.is_invertible() {
            self.error("only invertible diagrams can be inverted", span);
            return None;
        }
        let diagram = match DiagramN::try_from(diagram.clone()) {
            Ok(diagram) => diagram,
            Err(_) => {
                self.error("only positive-dimensional diagrams can be inverted", span);
                return None;
            }
        };
        Some(diagram.inverse().into())
    }

    fn compile_composition(
        &mut self,
        terms: &[Expr],
        span: Span,
        scope: &Scope,
    ) -> Option<Diagram> {
        let first = self.compile_expr(&terms[0], scope)?;
        let mut diagram = match DiagramN::try_from(first.clone()) {
            Ok(diagram) => diagram,
            Err(_) => {
                self.error(
                    "only positive-dimensional diagrams can be composed",
                    terms[0].span(),
                );
                return None;
            }
        };

        for term in &terms[1..] {
            let next = self.compile_expr(term, scope)?;
            let next = match DiagramN::try_from(next.clone()) {
                Ok(next) => next,
                Err(_) => {
                    self.error(
                        "only positive-dimensional diagrams can be composed",
                        term.span(),
                    );
                    return None;
                }
            };
            if diagram.dimension() != next.dimension() {
                self.error("composed diagrams must have the same dimension", span);
                return None;
            }
            if diagram.target() != next.source() {
                self.error(
                    "target of one term must match source of the next term",
                    span,
                );
                return None;
            }
            let cospans = diagram
                .cospans()
                .iter()
                .cloned()
                .chain(next.cospans().iter().cloned())
                .collect();
            diagram = DiagramN::new(diagram.source(), cospans);
        }

        Some(diagram.into())
    }

    fn expression_invertibility(&self, expr: &Expr, scope: &Scope) -> Option<Invertibility> {
        match expr {
            Expr::Name { name, .. } => {
                let resolved = self.resolve_name(name, scope);
                self.symbols
                    .get(&resolved)
                    .map(|symbol| symbol.invertibility)
            }
            Expr::Identity { .. } | Expr::Inverse { .. } => Some(Invertibility::Invertible),
            Expr::Contract { .. } => None,
            Expr::Compose { terms, .. } => terms
                .iter()
                .filter_map(|term| self.expression_invertibility(term, scope))
                .min(),
        }
    }

    fn finish(&mut self) -> Option<ProofState> {
        if self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            return None;
        }

        let workspace = self.selected_diagram.clone().map(Workspace::new);

        Some(ProofState {
            signature: self.signature.clone(),
            workspace,
            metadata: self.metadata.clone(),
            boundary: None,
            stash: Default::default(),
        })
    }

    fn next_generator(&mut self, dimension: usize) -> Generator {
        let generator = Generator::new(self.next_generator_id, dimension);
        self.next_generator_id += 1;
        generator
    }

    fn insert_symbol(&mut self, name: String, diagram: Diagram, invertibility: Invertibility) {
        let generator = diagram.max_generator().generator;
        let symbol = Symbol {
            info: SymbolInfo {
                name: name.clone(),
                dimension: diagram.dimension(),
                generator,
            },
            diagram,
            invertibility,
        };
        self.symbols.insert(name, symbol);
    }

    fn declared_name(&self, name: &str, scope: &Scope) -> String {
        if let Some(prefix) = &scope.prefix {
            format!("{prefix}.{name}")
        } else {
            name.to_owned()
        }
    }

    fn resolve_name(&self, name: &str, scope: &Scope) -> String {
        scope
            .aliases
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_owned())
    }

    fn resolve_instance_name(&self, name: &str, scope: &Scope) -> String {
        scope
            .instance_aliases
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_owned())
    }

    fn format_expr(&self, expr: &Expr, scope: &Scope) -> String {
        match expr {
            Expr::Name { name, .. } => self.resolve_name(name, scope),
            Expr::Identity { expr, .. } => format!("id({})", self.format_expr(expr, scope)),
            Expr::Inverse { expr, .. } => format!("inv({})", self.format_expr(expr, scope)),
            Expr::Contract { expr, bias, .. } => {
                let bias = bias
                    .map(|bias| match bias {
                        ContractionBias::Lower => "lower",
                        ContractionBias::Higher => "higher",
                        ContractionBias::Same => "same",
                    })
                    .map_or(String::new(), |bias| format!(", {bias}"));
                format!("contract({}{bias})", self.format_expr(expr, scope))
            }
            Expr::Compose { terms, .. } => terms
                .iter()
                .map(|term| match term {
                    Expr::Compose { .. } => format!("({})", self.format_expr(term, scope)),
                    _ => self.format_expr(term, scope),
                })
                .collect::<Vec<_>>()
                .join(" * "),
        }
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADJUNCTION: &str = r#"
title "Adjunction";
cell A;
cell B;

struct Adjunction(A: cell<0>, B: cell<0>) {
  cell F: A -> B;
  cell G: B -> A;
  cell unit: id(A) -> F * G;
  cell counit: G * F -> id(B);
}

use Adjunction(A, B) as adj;
show adj.unit;
"#;

    #[test]
    fn parses_comments_and_declaration_use() {
        let ast = parse(
            r#"
            // A small category-like shape.
            cell A;
            cell B;
            struct Pair(A: cell<0>, B: cell<0>) { cell f: A -> B; }
            use Pair(A, B) as pair;
            show pair.f;
            "#,
        )
        .unwrap();
        assert_eq!(ast.statements.len(), 5);
    }

    #[test]
    fn compiles_adjunction_struct() {
        let result = compile(ADJUNCTION, CompileOptions::default());
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        let names: Vec<_> = result.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"adj.F"));
        assert!(names.contains(&"adj.G"));
        assert!(names.contains(&"adj.unit"));
        assert!(names.contains(&"adj.counit"));
        assert_eq!(result.selected.as_deref(), Some("adj.unit"));
    }

    #[test]
    fn property_is_applicative_schema_alias() {
        let result = compile(
            r#"
            cell A;

            property Pointed(X: cell<0>) {
              cell loop: X -> X;
            }

            use Pointed(A) as first;
            use Pointed(A) as second;
            show second.loop;
            "#,
            CompileOptions::default(),
        );
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        let names: Vec<_> = result.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"first.loop"));
        assert!(!names.contains(&"second.loop"));
        assert_eq!(result.selected.as_deref(), Some("first.loop"));
    }

    #[test]
    fn struct_instantiations_are_generative() {
        let result = compile(
            r#"
            cell A;

            struct Pointed(X: cell<0>) {
              cell loop: X -> X;
            }

            use Pointed(A) as first;
            use Pointed(A) as second;
            show second.loop;
            "#,
            CompileOptions::default(),
        );
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        let names: Vec<_> = result.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"first.loop"));
        assert!(names.contains(&"second.loop"));
        assert_eq!(result.selected.as_deref(), Some("second.loop"));
    }

    #[test]
    fn use_with_fills_struct_fields_from_existing_symbols() {
        let result = compile(
            r#"
            cell A;
            cell existing: A -> A;

            struct Endomorphism(X: cell<0>) {
              cell map: X -> X;
            }

            use Endomorphism(A) as endo with {
              map = existing;
            }

            show endo.map;
            "#,
            CompileOptions::default(),
        );
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        let names: Vec<_> = result.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"existing"));
        assert!(!names.contains(&"endo.map"));
        assert_eq!(result.selected.as_deref(), Some("existing"));
    }

    #[test]
    fn use_with_rejects_directed_symbol_for_invertible_field() {
        let result = compile(
            r#"
            cell A;
            cell existing: A -> A;

            struct Idempotent(X: cell<0>) {
              cell map: X -> X;
              cell square: map * map <-> map;
            }

            use Idempotent(A) as bad with {
              map = existing;
              square = existing;
            }
            "#,
            CompileOptions::default(),
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("expects an invertible cell")));
    }

    #[test]
    fn struct_parameters_project_existing_fields() {
        let result = compile(
            r#"
            cell A;

            struct Idempotent(X: cell<0>) {
              cell e: X -> X;
              cell square: e * e <-> e;
            }

            struct Split(I: Idempotent) {
              cell witness: I.e -> I.e;
            }

            use Idempotent(A) as idem;
            use Split(idem) as split;
            show split.witness;
            "#,
            CompileOptions::default(),
        );
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        assert_eq!(result.selected.as_deref(), Some("split.witness"));
    }

    #[test]
    fn show_accepts_composed_macro_expressions() {
        for show in [
            "show (first.left * second.left);",
            "show first.left * second.left;",
        ] {
            let source = [
                r#"title "Macro Composition";
cell A;
cell B;
cell C;

macro Span(A: cell<0>, B: cell<0>) {
  cell left: A -> B;
  cell right: B -> A;
  cell witness: id(A) -> left * right;
}

use Span(A, B) as first;
use Span(B, C) as second;
"#,
                show,
                "\n",
            ]
            .concat();

            let result = compile(&source, CompileOptions::default());
            assert!(result.is_ok(), "{:?}", result.diagnostics);
            assert_eq!(result.selected.as_deref(), Some("first.left * second.left"));

            let workspace = result
                .proof
                .and_then(|proof| proof.workspace)
                .expect("composed show expression should create a workspace");
            workspace
                .diagram
                .check(true)
                .expect("shown composed diagram should validate");
            assert_eq!(workspace.diagram.dimension(), 1);
        }
    }

    #[test]
    fn compiles_invertible_cells_and_inverse_expressions() {
        let result = compile(
            "cell A; cell B; cell f: A <-> B; show inv(f);",
            CompileOptions::default(),
        );
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        assert_eq!(result.selected.as_deref(), Some("inv(f)"));

        let proof = result.proof.expect("invertible program should compile");
        let f = proof
            .signature
            .iter()
            .find(|info| info.name == "f")
            .expect("f should be in the signature");
        assert_eq!(f.invertibility, Invertibility::Invertible);

        let workspace = proof
            .workspace
            .expect("inverse show should select a workspace");
        workspace
            .diagram
            .check(true)
            .expect("inverse diagram should validate");
        assert_eq!(workspace.diagram.dimension(), 1);
    }

    #[test]
    fn invertible_composites_are_invertible() {
        let result = compile(
            "cell A; cell B; cell C; cell f: A <-> B; cell g: B <-> C; show inv(f * g);",
            CompileOptions::default(),
        );
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        assert_eq!(result.selected.as_deref(), Some("inv(f * g)"));
    }

    #[test]
    fn rejects_inverse_of_directed_diagrams() {
        let result = compile(
            "cell A; cell B; cell C; cell f: A <-> B; cell g: B -> C; show inv(f * g);",
            CompileOptions::default(),
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("only invertible diagrams")));
    }

    #[test]
    fn proves_inverse_cancellation_without_adding_axiom() {
        let result = compile(
            "cell A; cell B; cell f: A <-> B; prove cancel: f * inv(f) -> id(A); show cancel;",
            CompileOptions::default(),
        );
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        assert_eq!(result.selected.as_deref(), Some("cancel"));

        let proof = result.proof.expect("cancellation proof should compile");
        let signature_names: Vec<_> = proof
            .signature
            .iter()
            .map(|info| info.name.as_str())
            .collect();
        assert_eq!(signature_names, vec!["A", "B", "f"]);

        let cancel = result
            .symbols
            .iter()
            .find(|symbol| symbol.name == "cancel")
            .expect("constructed proof should still be a DSL symbol");
        assert_eq!(cancel.dimension, 2);

        let workspace = proof
            .workspace
            .expect("constructed proof should be selectable");
        workspace
            .diagram
            .check(true)
            .expect("constructed proof diagram should validate");
    }

    #[test]
    fn invertible_constructed_proofs_can_be_reused_as_inverses() {
        let result = compile(
            "cell A; cell B; cell f: A <-> B; prove cancel: f * inv(f) <-> id(A); construct undo: id(A) -> f * inv(f) { attach inv(cancel); } show undo;",
            CompileOptions::default(),
        );
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        assert!(result.symbols.iter().any(|symbol| symbol.name == "undo"));
    }

    #[test]
    fn rejects_inverse_of_directed_constructed_proofs() {
        let result = compile(
            "cell A; cell B; cell f: A <-> B; prove cancel: f * inv(f) -> id(A); show inv(cancel);",
            CompileOptions::default(),
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("only invertible diagrams")));
    }

    #[test]
    fn rejects_unsupported_constructed_proofs() {
        let result = compile(
            "cell A; cell B; cell f: A -> B; prove impossible: f -> id(A); show impossible;",
            CompileOptions::default(),
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("could not construct proof")));
    }

    #[test]
    fn compiles_paper_action_replay_source() {
        let result = compile(
            r#"actions [
              "CreateGeneratorZero",
              {"SelectGenerator":{"id":0,"dimension":0}},
              {"SetBoundary":"Source"},
              "CreateGeneratorZero",
              {"SelectGenerator":{"id":1,"dimension":0}},
              {"SetBoundary":"Target"}
            ]"#,
            CompileOptions::default(),
        );
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        let proof = result.proof.expect("action replay should produce proof");
        let dimensions: Vec<_> = proof
            .signature
            .iter()
            .map(|info| info.diagram.dimension())
            .collect();
        assert_eq!(dimensions, vec![0, 0, 1]);
    }

    #[test]
    fn compiles_signature_folders_and_construct_alias() {
        let result = compile(
            r#"
            cell A;
            cell B;
            cell f: A <-> B;

            folder Equivalences {
              construct cancel: f * inv(f) -> id(A);
              cell witness: id(A) -> f * inv(f);
            }

            show cancel;
            "#,
            CompileOptions::default(),
        );
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        assert_eq!(result.selected.as_deref(), Some("cancel"));

        let proof = result.proof.expect("folder program should compile");
        let tree = proof.signature.as_tree();
        let folder = tree
            .iter()
            .find_map(|(node, item)| match item.inner() {
                SignatureItem::Folder(info) if info.name == "Equivalences" => Some(node),
                _ => None,
            })
            .expect("folder should be present in signature");
        let child_names: Vec<_> = tree
            .with(folder, |item| item.children().collect::<Vec<_>>())
            .unwrap()
            .into_iter()
            .filter_map(|node| {
                tree.with(node, |item| match item.inner() {
                    SignatureItem::Item(info) => Some(info.name.clone()),
                    SignatureItem::Folder(_) => None,
                })
                .flatten()
            })
            .collect();
        assert_eq!(child_names, vec!["witness"]);
        assert!(result.symbols.iter().any(|symbol| symbol.name == "cancel"));
    }

    #[test]
    fn adjunction_comparison_maps_are_not_signature_axioms() {
        let preset = presets::get("uniqueness-of-adjunctions").unwrap();
        let result = compile(preset.source, CompileOptions::default());
        assert!(result.is_ok(), "{:?}", result.diagnostics);

        let proof = result.proof.expect("preset should compile");
        let signature_names: Vec<_> = proof
            .signature
            .iter()
            .map(|info| info.name.as_str())
            .collect();
        assert!(!signature_names.contains(&"to_second"));
        assert!(!signature_names.contains(&"to_first"));
        assert!(result
            .symbols
            .iter()
            .any(|symbol| symbol.name == "to_second"));
        assert!(result
            .symbols
            .iter()
            .any(|symbol| symbol.name == "to_first"));
        assert!(preset.source.contains("attach inv(to_second);"));
    }

    #[test]
    fn eckmann_hilton_preset_constructs_commutativity_without_axiom() {
        let preset = presets::get("eckmann-hilton").unwrap();
        let result = compile(preset.source, CompileOptions::default());
        assert!(result.is_ok(), "{:?}", result.diagnostics);
        assert_eq!(result.selected.as_deref(), Some("commute"));

        let proof = result.proof.expect("preset should compile");
        let signature_names: Vec<_> = proof
            .signature
            .iter()
            .map(|info| info.name.as_str())
            .collect();
        assert_eq!(signature_names, vec!["X", "alpha", "beta"]);
        assert!(result.symbols.iter().any(|symbol| symbol.name == "commute"));
        assert!(result
            .symbols
            .iter()
            .any(|symbol| symbol.name == "alpha_beta_to_horizontal"));
        assert!(result
            .symbols
            .iter()
            .any(|symbol| symbol.name == "beta_alpha_to_horizontal"));

        let workspace = proof
            .workspace
            .expect("Eckmann-Hilton proof should be selectable");
        workspace
            .diagram
            .check(true)
            .expect("Eckmann-Hilton proof diagram should validate");
        assert_eq!(workspace.diagram.dimension(), 3);
    }

    #[test]
    fn rejects_duplicate_names() {
        let result = compile("cell A; cell A;", CompileOptions::default());
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("duplicate symbol")));
    }

    #[test]
    fn rejects_dimension_mismatch() {
        let result = compile(
            "cell A; cell B; cell f: A -> B; cell bad: A -> f;",
            Default::default(),
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("source dimension")));
    }

    #[test]
    fn rejects_recursive_declaration() {
        let result = compile(
            "cell A; schema Loop(A: cell<0>) { use Loop(A) as next; } use Loop(A) as loop;",
            Default::default(),
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("recursive declaration")));
    }

    #[test]
    fn compiles_every_builtin_preset() {
        for preset in presets::PRESETS {
            let result = compile(preset.source, CompileOptions::default());
            assert!(
                result.is_ok(),
                "preset `{}` failed: {:?}",
                preset.id,
                result.diagnostics
            );
            let proof = result.proof.expect("compiled preset should have a proof");
            for generator in proof.signature.iter() {
                generator
                    .diagram
                    .check(true)
                    .expect("preset signature diagram should validate");
            }
            if let Some(workspace) = proof.workspace {
                workspace
                    .diagram
                    .check(true)
                    .expect("preset workspace diagram should validate");
            }
        }
    }
}
