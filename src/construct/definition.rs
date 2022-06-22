//! Definition is a construct that occurs in the [flow] content type.
//!
//! They’re formed with the following BNF:
//!
//! ```bnf
//! definition ::= label ':' whitespace destination [ whitespace title ] [ space_or_tab ]
//!
//! ; Restriction: maximum `999` codes allowed between brackets.
//! ; Restriction: no blank lines.
//! ; Restriction: at least 1 non-space and non-eol code must exist.
//! label ::= '[' *( label_text | label_escape ) ']'
//! label_text ::= code - '[' - '\\' - ']'
//! label_escape ::= '\\' [ '[' | '\\' | ']' ]
//!
//! destination ::= destination_enclosed | destination_raw
//! destination_enclosed ::= '<' *( destination_enclosed_text | destination_enclosed_escape ) '>'
//! destination_enclosed_text ::= code - '<' - '\\' - '>' - eol
//! destination_enclosed_escape ::= '\\' [ '<' | '\\' | '>' ]
//! destination_raw ::= 1*( destination_raw_text | destination_raw_escape )
//! ; Restriction: unbalanced `)` characters are not allowed.
//! destination_raw_text ::= code - '\\' - ascii_control - space_or_tab - eol
//! destination_raw_escape ::= '\\' [ '(' | ')' | '\\' ]
//!
//! ; Restriction: no blank lines.
//! ; Restriction: markers must match (in case of `(` with `)`).
//! title ::= marker [  *( code - '\\' | '\\' [ marker ] ) ] marker
//! marker ::= '"' | '\'' | '('
//!
//! whitespace ::= eol *whitespace | 1*space_or_tab [ eol *whitespace ]
//! space_or_tab ::= ' ' | '\t'
//! ```
//!
//! Definitions in markdown do not, on their own, relate to anything in HTML.
//! When connected with a link (reference), they together relate to the `<a>`
//! element in HTML.
//! The definition forms its `href`, and optionally `title`, attributes.
//! See [*§ 4.5.1 The `a` element*][html] in the HTML spec for more info.
//!
//! The `label`, `destination`, and `title` parts are interpreted as the
//! [string][] content type.
//! That means that [character escapes][character_escape] and
//! [character references][character_reference] are allowed.
//!
//! For info on how to encode characters in URLs, see
//! [`partial_destination`][destination].
//! For info on how to characters are encoded as `href` on `<a>` or `src` on
//! `<img>` when compiling, see
//! [`sanitize_uri`][sanitize_uri].
//!
//! ## Tokens
//!
//! *   [`Definition`][TokenType::Definition]
//! *   [`DefinitionMarker`][TokenType::DefinitionMarker]
//! *   [`DefinitionLabel`][TokenType::DefinitionLabel]
//! *   [`DefinitionLabelMarker`][TokenType::DefinitionLabelMarker]
//! *   [`DefinitionLabelString`][TokenType::DefinitionLabelString]
//! *   [`DefinitionDestination`][TokenType::DefinitionDestination]
//! *   [`DefinitionDestinationLiteral`][TokenType::DefinitionDestinationLiteral]
//! *   [`DefinitionDestinationLiteralMarker`][TokenType::DefinitionDestinationLiteralMarker]
//! *   [`DefinitionDestinationRaw`][TokenType::DefinitionDestinationRaw]
//! *   [`DefinitionDestinationString`][TokenType::DefinitionDestinationString]
//! *   [`DefinitionTitle`][TokenType::DefinitionTitle]
//! *   [`DefinitionTitleMarker`][TokenType::DefinitionTitleMarker]
//! *   [`DefinitionTitleString`][TokenType::DefinitionTitleString]
//! *   [`LineEnding`][TokenType::LineEnding]
//! *   [`SpaceOrTab`][TokenType::SpaceOrTab]
//!
//! ## References
//!
//! *   [`definition.js` in `micromark`](https://github.com/micromark/micromark/blob/main/packages/micromark-core-commonmark/dev/lib/definition.js)
//! *   [*§ 4.7 Link reference definitions* in `CommonMark`](https://spec.commonmark.org/0.30/#link-reference-definitions)
//!
//! [flow]: crate::content::flow
//! [string]: crate::content::string
//! [character_escape]: crate::construct::character_escape
//! [character_reference]: crate::construct::character_reference
//! [destination]: crate::construct::partial_destination
//! [sanitize_uri]: crate::util::sanitize_uri
//! [html]: https://html.spec.whatwg.org/multipage/text-level-semantics.html#the-a-element
//!
//! <!-- To do: link link (reference) -->
//!
//! <!-- To do: describe how references and definitions match -->

use crate::construct::{
    partial_destination::{start as destination, Options as DestinationOptions},
    partial_label::{start as label, Options as LabelOptions},
    partial_space_or_tab::space_or_tab,
    partial_title::{start as title, Options as TitleOptions},
};
use crate::tokenizer::{Code, State, StateFnResult, TokenType, Tokenizer};

/// At the start of a definition.
///
/// ```markdown
/// |[a]: b "c"
/// ```
pub fn start(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    tokenizer.enter(TokenType::Definition);
    tokenizer.attempt_opt(space_or_tab(), before)(tokenizer, code)
}

/// At the start of a definition, after whitespace.
///
/// ```markdown
/// |[a]: b "c"
/// ```
pub fn before(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    match code {
        Code::Char('[') => tokenizer.go(
            |t, c| {
                label(
                    t,
                    c,
                    LabelOptions {
                        label: TokenType::DefinitionLabel,
                        marker: TokenType::DefinitionLabelMarker,
                        string: TokenType::DefinitionLabelString,
                    },
                )
            },
            label_after,
        )(tokenizer, code),
        _ => (State::Nok, None),
    }
}

/// After the label of a definition.
///
/// ```markdown
/// [a]|: b "c"
/// ```
fn label_after(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    // To do: get the identifier:
    // identifier = normalizeIdentifier(
    //   self.sliceSerialize(self.events[self.events.length - 1][1]).slice(1, -1)
    // )

    match code {
        Code::Char(':') => {
            tokenizer.enter(TokenType::DefinitionMarker);
            tokenizer.consume(code);
            tokenizer.exit(TokenType::DefinitionMarker);
            (
                State::Fn(Box::new(
                    tokenizer.attempt_opt(space_or_tab(), marker_after),
                )),
                None,
            )
        }
        _ => (State::Nok, None),
    }
}

/// After the marker, after whitespace.
///
/// ```markdown
/// [a]: |b "c"
///
/// [a]: |␊
///  b "c"
/// ```
fn marker_after(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    match code {
        Code::CarriageReturnLineFeed | Code::Char('\r' | '\n') => {
            tokenizer.enter(TokenType::LineEnding);
            tokenizer.consume(code);
            tokenizer.exit(TokenType::LineEnding);
            (
                State::Fn(Box::new(
                    tokenizer.attempt_opt(space_or_tab(), destination_before),
                )),
                None,
            )
        }
        _ => destination_before(tokenizer, code),
    }
}

/// Before a destination.
///
/// ```markdown
/// [a]: |b "c"
///
/// [a]:
///  |b "c"
/// ```
fn destination_before(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    let event = tokenizer.events.last().unwrap();

    // Whitespace.
    if (event.token_type == TokenType::LineEnding || event.token_type == TokenType::SpaceOrTab)
    // Blank line not ok.
        && !matches!(
        code,
        Code::None | Code::CarriageReturnLineFeed | Code::Char('\r' | '\n')
    ) {
        tokenizer.go(
            |t, c| {
                destination(
                    t,
                    c,
                    DestinationOptions {
                        limit: usize::MAX,
                        destination: TokenType::DefinitionDestination,
                        literal: TokenType::DefinitionDestinationLiteral,
                        marker: TokenType::DefinitionDestinationLiteralMarker,
                        raw: TokenType::DefinitionDestinationRaw,
                        string: TokenType::DefinitionDestinationString,
                    },
                )
            },
            destination_after,
        )(tokenizer, code)
    } else {
        (State::Nok, None)
    }
}

/// After a destination.
///
/// ```markdown
/// [a]: b| "c"
///
/// [a]: b| ␊
///  "c"
/// ```
fn destination_after(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    tokenizer.attempt_opt(title_before, after)(tokenizer, code)
}

/// After a definition.
///
/// ```markdown
/// [a]: b|
/// [a]: b "c"|
/// ```
fn after(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    tokenizer.attempt_opt(space_or_tab(), after_whitespace)(tokenizer, code)
}

/// After a definition, after optional whitespace.
///
/// ```markdown
/// [a]: b |
/// [a]: b "c"|
/// ```
fn after_whitespace(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    match code {
        Code::None | Code::CarriageReturnLineFeed | Code::Char('\r' | '\n') => {
            tokenizer.exit(TokenType::Definition);
            (State::Ok, Some(vec![code]))
        }
        _ => (State::Nok, None),
    }
}

/// After a destination, presumably before a title.
///
/// ```markdown
/// [a]: b| "c"
///
/// [a]: b| ␊
///  "c"
/// ```
fn title_before(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    tokenizer.attempt_opt(space_or_tab(), title_before_after_optional_whitespace)(tokenizer, code)
}

/// Before a title, after optional whitespace.
///
/// ```markdown
/// [a]: b |"c"
///
/// [a]: b |␊
///  "c"
/// ```
fn title_before_after_optional_whitespace(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    match code {
        Code::CarriageReturnLineFeed | Code::Char('\r' | '\n') => {
            tokenizer.enter(TokenType::LineEnding);
            tokenizer.consume(code);
            tokenizer.exit(TokenType::LineEnding);
            (
                State::Fn(Box::new(
                    tokenizer.attempt_opt(space_or_tab(), title_before_marker),
                )),
                None,
            )
        }
        _ => title_before_marker(tokenizer, code),
    }
}

/// Before a title, after a line ending.
///
/// ```markdown
/// [a]: b␊
/// | "c"
/// ```
fn title_before_marker(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    let event = tokenizer.events.last().unwrap();

    if event.token_type == TokenType::LineEnding || event.token_type == TokenType::SpaceOrTab {
        tokenizer.go(
            |t, c| {
                title(
                    t,
                    c,
                    TitleOptions {
                        title: TokenType::DefinitionTitle,
                        marker: TokenType::DefinitionTitleMarker,
                        string: TokenType::DefinitionTitleString,
                    },
                )
            },
            title_after,
        )(tokenizer, code)
    } else {
        (State::Nok, None)
    }
}

/// After a title.
///
/// ```markdown
/// [a]: b "c"|
///
/// [a]: b␊
/// "c"|
/// ```
fn title_after(tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    tokenizer.attempt_opt(space_or_tab(), title_after_after_optional_whitespace)(tokenizer, code)
}

/// After a title, after optional whitespace.
///
/// ```markdown
/// [a]: b "c"|
///
/// [a]: b "c" |
/// ```
fn title_after_after_optional_whitespace(_tokenizer: &mut Tokenizer, code: Code) -> StateFnResult {
    match code {
        Code::None | Code::CarriageReturnLineFeed | Code::Char('\r' | '\n') => {
            (State::Ok, Some(vec![code]))
        }
        _ => (State::Nok, None),
    }
}
