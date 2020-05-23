/// S-Expression lexer implementation.
///
/// This isn't using any look-ahead yet and so always interprets
/// (.symbol) as ( DOT SYMBOL )
use crate::error::{err_lexer, spos, RuntimeError, SourcePos};

// key characters
const OPEN_PAREN: char = '(';
const CLOSE_PAREN: char = ')';
const SPACE: char = ' ';
const TAB: char = '\t';
const CR: char = '\r';
const LF: char = '\n';
const DOT: char = '.';
const DOUBLE_QUOTE: char = '"';
const SINGLE_QUOTE: char = '\'';

#[derive(Debug, PartialEq)]
pub enum TokenType {
    OpenParen,
    CloseParen,
    Symbol(String),
    Dot,
    Text(String),
    Quote,
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub pos: SourcePos,
    pub token: TokenType,
}

impl Token {
    fn new(pos: SourcePos, token: TokenType) -> Token {
        Token {
            pos: pos,
            token: token,
        }
    }
}

// tokenize a String
pub fn tokenize(input: &str) -> Result<Vec<Token>, RuntimeError> {
    use self::TokenType::*;

    // characters that terminate a symbol
    let terminating = [OPEN_PAREN, CLOSE_PAREN, SPACE, TAB, CR, LF, DOUBLE_QUOTE];
    let is_terminating = |c: char| terminating.iter().any(|t| c == *t);

    // return value
    let mut tokens = Vec::new();

    // start line numbering at 1, the first character of each line being number 0
    let mut lineno = 1;
    let mut charno = 0;

    let mut chars = input.chars();
    let mut current = chars.next();

    loop {
        match current {
            Some(TAB) => {
                return Err(err_lexer(
                    spos(lineno, charno),
                    "tabs are not valid whitespace",
                ));
            }

            Some(SPACE) => current = chars.next(),

            Some(CR) => {
                current = chars.next();

                // consume \n if it follows \r
                if let Some(LF) = current {
                    current = chars.next();
                }

                lineno += 1;
                charno = 0;
                continue;
            }

            Some(LF) => {
                current = chars.next();
                lineno += 1;
                charno = 0;
                continue;
            }

            // this is not correct because it doesn't allow for a . to begin a number
            // or a symbol. Will have to fix later.
            Some(DOT) => {
                tokens.push(Token::new(spos(lineno, charno), Dot));
                current = chars.next();
            }

            Some(OPEN_PAREN) => {
                tokens.push(Token::new(spos(lineno, charno), OpenParen));
                current = chars.next();
            }

            Some(CLOSE_PAREN) => {
                tokens.push(Token::new(spos(lineno, charno), CloseParen));
                current = chars.next();
            }

            Some(DOUBLE_QUOTE) => {
                let text_begin = charno;

                let mut text = String::from("");

                loop {
                    current = chars.next();
                    if let Some(c) = current {
                        if c == DOUBLE_QUOTE {
                            current = chars.next();
                            charno += 1;
                            break;
                        } else {
                            text.push(c);
                            charno += 1;
                        }
                    } else {
                        return Err(err_lexer(spos(lineno, charno), "Unterminated string"));
                    }
                }

                tokens.push(Token::new(spos(lineno, text_begin), Text(text)))
            }

            Some(SINGLE_QUOTE) => {
                tokens.push(Token::new(spos(lineno, charno), Quote));
                current = chars.next();
            }

            Some(non_terminating) => {
                let symbol_begin = charno;

                let mut symbol = String::from("");
                symbol.push(non_terminating);

                // consume symbol
                loop {
                    current = chars.next();
                    if let Some(c) = current {
                        if is_terminating(c) {
                            break;
                        } else {
                            symbol.push(c);
                            charno += 1;
                        }
                    } else {
                        break;
                    }
                }

                // complete symbol
                tokens.push(Token::new(spos(lineno, symbol_begin), Symbol(symbol)));
            }

            // EOL
            None => break,
        }

        charno += 1;
    }

    Ok(tokens)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lexer_empty_string() {
        if let Ok(tokens) = tokenize("") {
            assert!(tokens.len() == 0);
        } else {
            assert!(false, "unexpected error");
        }
    }

    #[test]
    fn lexer_one_line() {
        if let Ok(tokens) = tokenize("(foo bar baz)") {
            assert!(tokens.len() == 5);
            assert_eq!(tokens[0], Token::new(spos(1, 0), TokenType::OpenParen));
            assert_eq!(
                tokens[1],
                Token::new(spos(1, 1), TokenType::Symbol(String::from("foo")))
            );
            assert_eq!(
                tokens[2],
                Token::new(spos(1, 5), TokenType::Symbol(String::from("bar")))
            );
            assert_eq!(
                tokens[3],
                Token::new(spos(1, 9), TokenType::Symbol(String::from("baz")))
            );
            assert_eq!(tokens[4], Token::new(spos(1, 12), TokenType::CloseParen));
        } else {
            assert!(false, "unexpected error");
        }
    }

    #[test]
    fn lexer_multi_line() {
        if let Ok(tokens) = tokenize("( foo\nbar\nbaz\n)") {
            assert!(tokens.len() == 5);
            assert_eq!(tokens[0], Token::new(spos(1, 0), TokenType::OpenParen));
            assert_eq!(
                tokens[1],
                Token::new(spos(1, 2), TokenType::Symbol(String::from("foo")))
            );
            assert_eq!(
                tokens[2],
                Token::new(spos(2, 0), TokenType::Symbol(String::from("bar")))
            );
            assert_eq!(
                tokens[3],
                Token::new(spos(3, 0), TokenType::Symbol(String::from("baz")))
            );
            assert_eq!(tokens[4], Token::new(spos(4, 0), TokenType::CloseParen));
        } else {
            assert!(false, "unexpected error");
        }
    }

    #[test]
    fn lexer_bad_whitespace() {
        if let Err(e) = tokenize("(foo\n\t(bar))") {
            if let Some(SourcePos { line, column }) = e.error_pos() {
                assert_eq!(line, 2);
                assert_eq!(column, 0);
            } else {
                assert!(false, "Expected error position");
            }
        } else {
            assert!(false, "expected ParseEvalError for tab character");
        }
    }

    #[test]
    fn lexer_text() {
        if let Ok(_tokens) = tokenize("(foo \"text\" bar)") {
            // TODO
        } else {
            assert!(false, "unexpected error")
        }
    }
}
