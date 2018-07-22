#![feature(nll, label_break_value, proc_macro_diagnostic)]

#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(speculate))]

extern crate proc_macro;
extern crate rustc_version;


use proc_macro::{
    Literal, Span, TokenStream, TokenTree,
    token_stream::IntoIter
};
use rustc_version::{version, Version};
use std::iter::Peekable;


type Tokens = Peekable<IntoIter>;

struct Condition(Option<(Version, Cmp)>, Option<(Version, Cmp)>);

enum Cmp {
    EQ, NE, GT, LT, GE, LE
}

macro_rules! emit {
    ( $diag: expr ) => ({
        $diag.emit();
        return None
    });
}

fn parse_version(first_token: Literal, tokens: &mut Tokens) -> Option<Version> {
    // Major
    let first = format!("{}", first_token);

    if let Some(i) = first.find('.') {
        let major: u64 = match first[..i].parse() {
            Ok(n) => n,
            Err(err) => emit!(
                first_token.span().error(format!("Cannot parse major version: {}.", err))
            )
        };

        let minor: u64 = match first[i+1..].parse() {
            Ok(n) => n,
            Err(err) => emit!(
                first_token.span().error(format!("Cannot parse minor version: {}.", err))
            )
        };

        let patch: u64 = 'patch: {
            match tokens.peek() {
                Some(TokenTree::Punct(op)) if op.as_char() == '.' => {
                    tokens.next();
                },

                _ => break 'patch 0
            }
            
            match tokens.next() {
                Some(TokenTree::Literal(lit)) => match format!("{}", lit).parse() {
                    Ok(n) => n,
                    Err(err) => emit!(
                        lit.span().error(format!("Expected numeric literal: {}.", err))
                    )
                },
                Some(other) => emit!(
                    other.span().error("Expected numeric literal.")
                ),
                None => 0
            }
        };

        Some(Version::new(major, minor, patch))
    } else {
        match first.parse() {
            Ok(major) => Some(Version::new(major, 0, 0)),
            Err(err) => emit!(
                first_token.span().error(format!("Expected numeric literal: {}.", err))
            )
        }
    }
}

fn parse_cmp(tokens: &mut Tokens, emit_if_end: bool) -> Option<Cmp> {
    match tokens.next() {
        Some(TokenTree::Punct(op)) => match op.as_char() {
            '!' => match tokens.next() {
                Some(TokenTree::Punct(ref op)) if op.as_char() == '=' => Some(Cmp::NE),
                Some(other) => emit!(other.span().error("Expected '=' character.")),
                None => emit!(op.span().error("Expected '=' character after '!'."))
            },

            '=' => match tokens.next() {
                Some(TokenTree::Punct(ref op)) if op.as_char() == '=' => Some(Cmp::EQ),
                Some(other) => emit!(other.span().error("Expected '=' character.")),
                None => emit!(op.span().error("Expected '=' character after '='."))
            },

            '<' => match tokens.peek() {
                Some(TokenTree::Punct(ref op)) if op.as_char() == '=' => {
                    tokens.next();
                    Some(Cmp::LE)
                },

                Some(_) => Some(Cmp::LT),
                
                None => if emit_if_end {
                    emit!(op.span().error("Unexpected end of condition."))
                } else {
                    None
                }
            },

            '>' => match tokens.peek() {
                Some(TokenTree::Punct(op)) if op.as_char() == '=' => {
                    tokens.next();
                    Some(Cmp::GE)
                },
                
                Some(_) => Some(Cmp::GT),
                
                None => if emit_if_end {
                    emit!(op.span().error("Unexpected end of condition."))
                } else {
                    None
                }
            },

            _ => emit!(op.span().error("Unknown comparison operator."))
        },
        
        Some(other) => emit!(other.span().error("Expected comparison operator.")),
        
        None => if emit_if_end {
            emit!(Span::call_site().error("Invalid condition."))
        } else {
            None
        }
    }
}

fn parse_condition(tokens: &mut Tokens) -> Option<Condition> {
    let left = match tokens.next() {
        Some(TokenTree::Ident(term)) => {
            // "x <= 0.0.0"
            let cmp = parse_cmp(tokens, true)?;
            let version = match tokens.next() {
                Some(TokenTree::Literal(lit)) => parse_version(lit, tokens)?,
                Some(other) => emit!(other.span().error("Expected version literal.")),
                None => emit!(term.span().error("Unexpected end of condition."))
            };

            return Some(Condition(None, Some((version, cmp))))
        },
        Some(TokenTree::Literal(lit)) => parse_version(lit, tokens)?,
        Some(other) => emit!(other.span().error("Expected version literal.")),
        None => emit!(Span::call_site().error("Invalid condition."))
    };

    let left_cmp = parse_cmp(tokens, true)?;

    match tokens.next() {
        Some(TokenTree::Ident(_)) => (),
        Some(other) => emit!(other.span().error("Expected identifier.")),
        None => emit!(Span::call_site().error("Invalid condition."))
    }

    let right_cmp = if let Some(cmp) = parse_cmp(tokens, false) {
        cmp
    } else {
        // "0.0.0 <= x"
        return Some(Condition(Some((left, left_cmp)), None))
    };

    let right = match tokens.next() {
        Some(TokenTree::Literal(lit)) => parse_version(lit, tokens)?,
        Some(other) => emit!(other.span().error("Expected version literal.")),
        None => emit!(Span::call_site().error("Invalid condition."))
    };

    // "0.0.0 <= x <= 0.0.0"
    Some(Condition(Some((left, left_cmp)), Some((right, right_cmp))))
}

fn is_condition_true(left: Version, cmp: Cmp, right: Version) -> bool {
    match cmp {
        Cmp::EQ => left == right,
        Cmp::NE => left != right,
        Cmp::GT => left >  right,
        Cmp::GE => left >= right,
        Cmp::LT => left <  right,
        Cmp::LE => left <= right,
    }
}

#[proc_macro_attribute]
pub fn rust_version(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut tokens = args.into_iter().peekable();

    // Parse condition
    let Condition(lhs, rhs) = match parse_condition(&mut tokens) {
        Some(condition) => condition,
        None => return TokenStream::new()
    };

    // Find current version
    let current = match version() {
        Ok(version) => version,
        Err(err) => {
            Span::call_site().error(
                format!("Unable to get current Rust version: {}.", err)
            ).emit();

            return TokenStream::new()
        }
    };

    // Interpret condition
    let accepted = match (lhs, rhs) {
        (Some((l, lcmp)), Some((r, rcmp))) => is_condition_true(l, lcmp, current.clone())
                                           && is_condition_true(current, rcmp, r),
        
        (Some((l, lcmp)), None) => is_condition_true(l, lcmp, current),
        (None, Some((r, rcmp))) => is_condition_true(current, rcmp, r),

        (None, None) => unreachable!()
    };

    if accepted {
        input
    } else {
        TokenStream::new()
    }
}

#[cfg(test)]
speculate! {
    macro_rules! assert_true {
        ($l: expr, $c: expr, $r: expr) => (
            assert!(is_condition_true(
                Version::parse($l).unwrap(), $c, Version::parse($r).unwrap()
            ))
        )
    }

    it "can interpret conditions correctly" {
        assert_true!("0.0.0", Cmp::EQ, "0.0.0");
        assert_true!("0.1.0", Cmp::LT, "0.1.1");
    }

}
