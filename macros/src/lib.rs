use convert_case::{Case, Casing};
use proc_macro::{TokenStream, TokenTree};
use quote::quote_spanned;

#[proc_macro_attribute]
pub fn message_types(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut stream = TokenStream::new();
    let mut attr = attr
        .into_iter()
        .collect::<Vec<TokenTree>>()
        .into_iter()
        .filter(|token| token.to_string() != ",")
        .peekable();

    let mut typ_option = attr.next();
    while let Some(ref typ) = typ_option {
        if let TokenTree::Ident(ident) = typ.clone() {
            let attr_string = format!("{}", ident);
            let span = typ.span().into();

            let return_type: Option<syn::Type> = {
                if let Some(next_type) = attr.peek() {
                    if let Some(inside) = extract_inside_parentheses(&next_type.to_string()) {
                        let return_type = syn::parse_str(&inside).unwrap();
                        let _ = attr.next();
                        Some(return_type)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let member = match attr_string.find('(') {
                Some(pos) => (&attr_string[..pos]).to_string(),
                None => attr_string,
            };

            let member_snake_case = format!("{}", member).to_case(Case::Snake);
            let member_pascal_case = format!("{}", member).to_case(Case::Pascal);

            let new_fn_name =
                syn::Ident::new(&format!("expect_message_{}", member_snake_case), span);

            let some_res = format!("{}", member_pascal_case);
            let some_res = syn::Ident::new(&some_res, span);

            let generated = match return_type {
                Some(return_type) => {
                    quote_spanned! {span=>
                        async fn #new_fn_name(&mut self) -> #return_type {
                            loop {
                                let message = self.expect_message().await;
                                if let Message::#some_res(inner) = message {
                                    return inner;
                                }
                                eprintln!("warning: recieved unexpected Message: {:?}", message);
                            }
                        }
                    }
                }
                None => {
                    quote_spanned! {span=>
                        async fn #new_fn_name(&mut self) {
                            loop {
                                let message = self.expect_message().await;
                                if message == Message::#some_res {
                                    return;
                                }
                                eprintln!("warning: recieved unexpected Message: {:?}", message);
                            }
                        }
                    }
                }
            };

            stream.extend(TokenStream::from(generated));
        }
        typ_option = attr.next();
    }
    stream.extend(item);
    stream
}

fn extract_inside_parentheses(s: &str) -> Option<String> {
    if s.starts_with('(') && s.ends_with(')') {
        Some(s[1..s.len() - 1].to_string())
    } else {
        None
    }
}
