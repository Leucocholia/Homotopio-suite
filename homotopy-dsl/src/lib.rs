use std::{
    collections::{BTreeMap, HashMap, HashSet},
    convert::TryFrom,
    str::FromStr,
};

use homotopy_core::{
    signature::Invertibility, Diagram, Diagram0, DiagramN, Generator, Orientation,
};
use homotopy_graphics::style::{Color, VertexShape};
use homotopy_model::proof::{
    generators::GeneratorInfo, Metadata, ProofState, Signature, SignatureItem, Workspace,
};
use serde::{Deserialize, Serialize};

pub mod presets;

const COLORS: &[&str] = &[
    "#2980b9", "#c0392b", "#f39c12", "#8e44ad", "#27ae60", "#f1c40f", "#6b7280", "#111827",
];

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
    Schema(SchemaDecl),
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
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub dimension: usize,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseDecl {
    pub schema: String,
    pub args: Vec<String>,
    pub alias: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    Name { name: String, span: Span },
    Identity { expr: Box<Expr>, span: Span },
    Compose { terms: Vec<Expr>, span: Span },
}

impl Expr {
    fn span(&self) -> Span {
        match self {
            Self::Name { span, .. } | Self::Identity { span, .. } | Self::Compose { span, .. } => {
                *span
            }
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
    Arrow,
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
                b'<' => self.single(TokenKind::Lt),
                b'>' => self.single(TokenKind::Gt),
                b'*' => self.single(TokenKind::Star),
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
            "schema" => TokenKind::Keyword("schema"),
            "macro" => TokenKind::Keyword("macro"),
            "use" => TokenKind::Keyword("use"),
            "as" => TokenKind::Keyword("as"),
            "show" => TokenKind::Keyword("show"),
            "id" => TokenKind::Keyword("id"),
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
            TokenKind::Keyword("schema") | TokenKind::Keyword("macro") => {
                self.parse_schema().map(Stmt::Schema)
            }
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
        let (source, target) = if self.eat(TokenKind::Colon).is_some() {
            let source = self.parse_expr()?;
            self.expect(TokenKind::Arrow, "expected `->` in cell declaration")?;
            let target = self.parse_expr()?;
            (Some(source), Some(target))
        } else {
            (None, None)
        };
        let end = self.expect(TokenKind::Semicolon, "expected `;` after cell declaration")?;
        Some(CellDecl {
            name,
            source,
            target,
            span: start.join(end.span),
        })
    }

    fn parse_schema(&mut self) -> Option<SchemaDecl> {
        let start = self.bump().span;
        let (name, _) = self.parse_name()?;
        self.expect(TokenKind::LParen, "expected `(` after schema name")?;
        let mut params = Vec::new();
        if !self.at(TokenKind::RParen) {
            loop {
                let (param_name, param_span) = self.parse_name()?;
                self.expect(TokenKind::Colon, "expected `:` after parameter name")?;
                self.expect_keyword("cell", "expected `cell<dimension>` parameter type")?;
                self.expect(TokenKind::Lt, "expected `<` in parameter type")?;
                let dimension = self.expect_number("expected parameter dimension")?;
                self.expect(TokenKind::Gt, "expected `>` in parameter type")?;
                params.push(Param {
                    name: param_name,
                    dimension,
                    span: param_span,
                });
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "expected `)` after parameters")?;
        self.expect(TokenKind::LBrace, "expected `{` before schema body")?;
        let mut body = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            if let Some(stmt) = self.parse_stmt() {
                body.push(stmt);
            } else {
                self.synchronize();
            }
        }
        let end = self.expect(TokenKind::RBrace, "expected `}` after schema body")?;
        Some(SchemaDecl {
            name,
            params,
            body,
            span: start.join(end.span),
        })
    }

    fn parse_use(&mut self) -> Option<UseDecl> {
        let start = self.bump().span;
        let (schema, _) = self.parse_name()?;
        self.expect(TokenKind::LParen, "expected `(` after schema name")?;
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
        self.expect(TokenKind::RParen, "expected `)` after schema arguments")?;
        self.expect_keyword("as", "expected `as` before schema alias")?;
        let (alias, _) = self.parse_name()?;
        let end = self.expect(TokenKind::Semicolon, "expected `;` after schema use")?;
        Some(UseDecl {
            schema,
            args,
            alias,
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
    prefix: Option<String>,
}

struct Compiler {
    signature: Signature,
    symbols: BTreeMap<String, Symbol>,
    schemas: HashMap<String, SchemaDecl>,
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
            schemas: HashMap::new(),
            diagnostics: Vec::new(),
            selected: None,
            selected_diagram: None,
            metadata,
            next_generator_id: 0,
            expansion_stack: Vec::new(),
        }
    }

    fn compile_ast(&mut self, ast: &Ast) {
        let mut seen_schemas = HashSet::new();
        for stmt in &ast.statements {
            if let Stmt::Schema(schema) = stmt {
                if !seen_schemas.insert(schema.name.clone()) {
                    self.error(format!("duplicate schema `{}`", schema.name), schema.span);
                } else {
                    self.schemas.insert(schema.name.clone(), schema.clone());
                }
            }
        }

        let mut scope = Scope::default();
        for stmt in &ast.statements {
            if matches!(stmt, Stmt::Schema(_)) {
                continue;
            }
            self.compile_stmt(stmt, &mut scope);
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
            Stmt::Schema(decl) => {
                self.error(
                    format!("nested schema `{}` is not supported in V1", decl.name),
                    decl.span,
                );
            }
        }
    }

    fn compile_cell(&mut self, decl: &CellDecl, scope: &mut Scope) {
        let public_name = self.declared_name(&decl.name, scope);
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
            invertibility: Invertibility::Directed,
            single_preview: true,
            color: Color::from_str(COLORS[generator.id % COLORS.len()]).unwrap(),
            shape: VertexShape::default(),
            diagram: diagram.clone(),
            name: public_name.clone(),
        };
        self.signature.insert_item(SignatureItem::Item(info));
        let symbol = Symbol {
            info: SymbolInfo {
                name: public_name.clone(),
                dimension: diagram.dimension(),
                generator,
            },
            diagram,
        };

        if scope.prefix.is_some() {
            scope.aliases.insert(decl.name.clone(), public_name.clone());
        }
        self.symbols.insert(public_name, symbol);
    }

    fn compile_use(&mut self, decl: &UseDecl, scope: &mut Scope) {
        let schema_name = self.resolve_name(&decl.schema, scope);
        let Some(schema) = self.schemas.get(&schema_name).cloned() else {
            self.error(format!("unknown schema `{}`", decl.schema), decl.span);
            return;
        };

        if self.expansion_stack.contains(&schema.name) {
            self.error(
                format!("recursive schema expansion involving `{}`", schema.name),
                decl.span,
            );
            return;
        }

        if schema.params.len() != decl.args.len() {
            self.error(
                format!(
                    "schema `{}` expects {} arguments but received {}",
                    schema.name,
                    schema.params.len(),
                    decl.args.len()
                ),
                decl.span,
            );
            return;
        }

        let public_alias = self.declared_name(&decl.alias, scope);
        let mut child = Scope {
            aliases: HashMap::new(),
            prefix: Some(public_alias),
        };

        for (param, arg) in schema.params.iter().zip(&decl.args) {
            let resolved_arg = self.resolve_name(arg, scope);
            let Some(symbol) = self.symbols.get(&resolved_arg) else {
                self.error(format!("unknown argument `{arg}`"), param.span);
                return;
            };
            if symbol.info.dimension != param.dimension {
                self.error(
                    format!(
                        "argument `{arg}` has dimension {}, expected {}",
                        symbol.info.dimension, param.dimension
                    ),
                    param.span,
                );
                return;
            }
            child.aliases.insert(param.name.clone(), resolved_arg);
        }

        self.expansion_stack.push(schema.name.clone());
        for stmt in &schema.body {
            self.compile_stmt(stmt, &mut child);
        }
        self.expansion_stack.pop();
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
            Expr::Compose { terms, span } => self.compile_composition(terms, *span, scope),
        }
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

    fn format_expr(&self, expr: &Expr, scope: &Scope) -> String {
        match expr {
            Expr::Name { name, .. } => self.resolve_name(name, scope),
            Expr::Identity { expr, .. } => format!("id({})", self.format_expr(expr, scope)),
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

schema Adjunction(A: cell<0>, B: cell<0>) {
  cell F: A -> B;
  cell G: B -> A;
  cell unit: id(A) -> F * G;
  cell counit: G * F -> id(B);
}

use Adjunction(A, B) as adj;
show adj.unit;
"#;

    #[test]
    fn parses_comments_and_schema_use() {
        let ast = parse(
            r#"
            // A small category-like shape.
            cell A;
            cell B;
            schema Pair(A: cell<0>, B: cell<0>) { cell f: A -> B; }
            use Pair(A, B) as pair;
            show pair.f;
            "#,
        )
        .unwrap();
        assert_eq!(ast.statements.len(), 5);
    }

    #[test]
    fn compiles_adjunction_schema() {
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
    fn rejects_recursive_schema() {
        let result = compile(
            "cell A; schema Loop(A: cell<0>) { use Loop(A) as next; } use Loop(A) as loop;",
            Default::default(),
        );
        assert!(!result.is_ok());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("recursive schema")));
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
