#![allow(clippy::needless_continue, clippy::option_if_let_else)]
use darling::FromDeriveInput;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Expr, LitStr, parse_macro_input};

use super::utils::validate_int_expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(command), supports(struct_unit, struct_named))]
struct SubcommandOpts {
    arity: Expr,
    first_key: Expr,
    last_key: Expr,
    step: Expr,
    summary: LitStr,
    complexity: LitStr,
    since: LitStr,
}

impl SubcommandOpts {
    pub fn validate(&self) -> syn::Result<()> {
        validate_int_expr(&self.arity)?;
        validate_int_expr(&self.first_key)?;
        validate_int_expr(&self.last_key)?;
        validate_int_expr(&self.step)?;
        Ok(())
    }
}

impl quote::ToTokens for SubcommandOpts {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let arity = &self.arity;
        let first_key = &self.first_key;
        let last_key = &self.last_key;
        let step = &self.step;
        let summary = &self.summary;
        let complexity = &self.complexity;
        let since = &self.since;

        tokens.extend(quote! {
            arity: #arity,
            first_key: #first_key,
            last_key: #last_key,
            step: #step,
            summary: #summary,
            complexity: #complexity,
            since: #since,
        });
    }
}

pub fn derive_subcommand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident.clone();

    let opts = match SubcommandOpts::from_derive_input(&input) {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
    };

    if let Err(e) = opts.validate() {
        return e.to_compile_error().into();
    }

    let name_tokens = LitStr::new(&ident.to_string().to_uppercase(), ident.span());

    let expanded = quote! {
        #[::async_trait::async_trait]
        impl crate::prelude::SubcommandTrait for #ident
        where
            #ident: crate::prelude::Handler,
        {
            fn name(&self) -> &'static str {
                #name_tokens
            }

            fn info(&self) -> crate::prelude::CommandInfo {
                crate::prelude::CommandInfo {
                   #opts
                }
            }

            async fn handle(
                &self,
                writer: &mut (dyn ::tokio::io::AsyncWrite + ::std::marker::Unpin + ::std::marker::Send),
                args: &mut crate::prelude::Args<'_>,
                session: & crate::session::Session,
            ) -> crate::prelude::Result<()> {
                <#ident as crate::prelude::Handler>::handle(self, writer, args, session).await
            }
        }
    };

    expanded.into()
}
