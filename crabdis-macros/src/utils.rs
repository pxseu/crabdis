use syn::spanned::Spanned;
use syn::{Expr, Result};

pub fn validate_int_expr(expr: &Expr) -> Result<()> {
    use syn::{BinOp, ExprBinary, ExprGroup, ExprLit, ExprParen, ExprUnary, Lit, UnOp};

    match expr {
        Expr::Lit(ExprLit { lit, .. }) => match lit {
            Lit::Int(_) => Ok(()),
            _ => Err(syn::Error::new(lit.span(), "expected an integer literal")),
        },

        Expr::Paren(ExprParen { expr, .. }) | Expr::Group(ExprGroup { expr, .. }) => {
            validate_int_expr(expr)
        }

        Expr::Unary(ExprUnary { op, expr, .. }) => match op {
            UnOp::Neg(_) => validate_int_expr(expr),
            _ => Err(syn::Error::new(op.span(), "only unary '-' is allowed")),
        },

        Expr::Binary(ExprBinary {
            left, op, right, ..
        }) => match op {
            BinOp::Add(_) | BinOp::Sub(_) | BinOp::Mul(_) | BinOp::Div(_) | BinOp::Rem(_) => {
                validate_int_expr(left)?;
                validate_int_expr(right)?;
                Ok(())
            }
            _ => Err(syn::Error::new(op.span(), "only + - * / % are allowed")),
        },

        _ => Err(syn::Error::new(
            expr.span(),
            "expected integer arithmetic (literals, parentheses, + - * / %, unary -)",
        )),
    }
}
