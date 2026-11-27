//! テスト名を文字列で書くための `describe!` マクロ（Jest / Playwright の describe / test 相当）。
//!
//! ```ignore
//! use meetup_test_macros::describe;
//!
//! describe!("認証", {
//!     describe!("新規登録", {
//!         test!("登録すると 201 とトークンが返る", async {
//!             // ...
//!         });
//!     });
//!     test!("同期のテスト", {
//!         assert_eq!(1 + 1, 2);
//!     });
//! });
//! ```
//!
//! - `describe!(名前, { … })` は `mod` に、`test!(名前, 本体)` はテスト関数に展開する。
//!   名前の文字列は識別子に変換する（識別子に使えない文字の並びは `_` 1 つにし、先頭・末尾のものは削る。
//!   数字など先頭に置けない文字で始まれば `_` を前置、キーワードと衝突すれば `_` を後置）。
//!   `cargo test` の表示は `認証::新規登録::登録すると_201_とトークンが返る` のようになる。
//!   同じ `describe!` の中で変換後の名前が重なったら、2 つ目の名前の位置でコンパイルエラーにする
//! - 本体が `async { … }` なら `#[tokio::test]`、ブロックなら `#[test]` を付ける。
//!   `test!` の前に `#[tokio::test(flavor = "multi_thread")]` などテスト属性（最後のセグメントが
//!   `test` か `…_test`、または `rstest` / `test_case`）を書けば、既定の属性は付けない。
//!   `#[ignore = "…"]` などの他の属性はそのまま関数に付く
//! - `describe!` の中にはヘルパー関数・`use`・定数なども書ける。各 `describe!` は `use super::*;` を持つ。
//!   生成した `mod` / `fn` は兄弟の名前を覆い隠すので、ヘルパーやクレートと同じ名前（`app` など）は付けない
//! - `test!` は `describe!` の中でだけ使える（`describe!` が展開する）

use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Expr, Ident, Item, ItemMacro, LitStr, Token, braced};

#[proc_macro]
pub fn describe(input: TokenStream) -> TokenStream {
    match syn::parse::<Describe>(input).and_then(|d| d.expand()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// `describe!("名前", { 項目… })` の中身
struct Describe {
    name: LitStr,
    items: Vec<Item>,
}

impl Parse for Describe {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let content;
        braced!(content in input);
        let mut items = Vec::new();
        while !content.is_empty() {
            items.push(content.parse()?);
        }
        let _ = input.parse::<Option<Token![,]>>()?;
        Ok(Self { name, items })
    }
}

impl Describe {
    fn expand(self) -> syn::Result<TokenStream2> {
        let ident = to_ident(&self.name)?;
        let doc = self.name.value();
        check_collisions(&self.items)?;
        let items = self
            .items
            .into_iter()
            .map(expand_item)
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(quote! {
            #[doc = #doc]
            #[allow(non_snake_case, clippy::module_inception)]
            mod #ident {
                #[allow(unused_imports)]
                use super::*;
                #(#items)*
            }
        })
    }
}

/// 同じ `describe!` 直下の `describe!` / `test!` で、変換後の識別子が重なっていないか
fn check_collisions(items: &[Item]) -> syn::Result<()> {
    let mut seen: HashMap<String, LitStr> = HashMap::new();
    for item in items {
        let Item::Macro(mac) = item else { continue };
        if !(mac.mac.path.is_ident("describe") || mac.mac.path.is_ident("test")) {
            continue;
        }
        let name: LitStr = mac.mac.parse_body_with(|input: ParseStream| {
            let name = input.parse()?;
            input.parse::<proc_macro2::TokenStream>()?;
            Ok(name)
        })?;
        let ident = to_ident(&name)?.to_string();
        if let Some(first) = seen.get(&ident) {
            return Err(syn::Error::new(
                name.span(),
                format!(
                    "「{}」は「{}」と同じ識別子 `{ident}` になります。名前を変えてください",
                    name.value(),
                    first.value()
                ),
            ));
        }
        seen.insert(ident, name);
    }
    Ok(())
}

/// `describe!` / `test!` の呼び出しだけを展開し、それ以外の項目はそのまま残す
fn expand_item(item: Item) -> syn::Result<TokenStream2> {
    let Item::Macro(mac) = item else {
        return Ok(quote!(#item));
    };
    if mac.mac.path.is_ident("describe") {
        if !mac.attrs.is_empty() {
            return Err(syn::Error::new_spanned(
                &mac.attrs[0],
                "describe! には属性を付けられません（test! に付けてください）",
            ));
        }
        return mac.mac.parse_body::<Describe>()?.expand();
    }
    if mac.mac.path.is_ident("test") {
        return expand_test(mac);
    }
    Ok(quote!(#mac))
}

/// `test!("名前", 本体)` の中身
struct TestCase {
    name: LitStr,
    body: Expr,
}

impl Parse for TestCase {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let body: Expr = input.parse()?;
        let _ = input.parse::<Option<Token![,]>>()?;
        Ok(Self { name, body })
    }
}

fn expand_test(mac: ItemMacro) -> syn::Result<TokenStream2> {
    let TestCase { name, body } = mac.mac.parse_body()?;
    let ident = to_ident(&name)?;
    let doc = name.value();
    let attrs = mac.attrs;
    let has_test_attr = attrs.iter().any(is_test_attr);
    let (asyncness, block, default_attr) = match body {
        Expr::Async(expr) if expr.attrs.is_empty() => {
            (quote!(async), expr.block, quote!(#[::tokio::test]))
        }
        Expr::Block(expr) if expr.attrs.is_empty() && expr.label.is_none() => {
            (quote!(), expr.block, quote!(#[test]))
        }
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "test! の本体は属性もラベルも無い `{ … }` か `async { … }` にしてください（属性は test! の前に書く）",
            ));
        }
    };
    let test_attr = if has_test_attr {
        quote!()
    } else {
        default_attr
    };
    Ok(quote! {
        #test_attr
        #(#attrs)*
        #[doc = #doc]
        #[allow(non_snake_case)]
        #asyncness fn #ident() #block
    })
}

/// テストとして登録する属性か（`#[test]` / `#[tokio::test(...)]` / `#[wasm_bindgen_test]` /
/// `#[rstest]` / `#[test_case(...)]` など）。これがあれば既定の属性は付けない
fn is_test_attr(attr: &Attribute) -> bool {
    attr.path().segments.last().is_some_and(|segment| {
        let name = segment.ident.to_string();
        name == "test" || name.ends_with("_test") || name == "rstest" || name == "test_case"
    })
}

/// テスト名の文字列を識別子にする
fn to_ident(name: &LitStr) -> syn::Result<Ident> {
    let text = sanitize(&name.value())
        .ok_or_else(|| syn::Error::new(name.span(), "名前に識別子として使える文字がありません"))?;
    Ok(Ident::new(&text, name.span()))
}

/// 識別子に使えない文字の並びを `_` 1 つに置き換える。先頭に置けない文字なら `_` を前置し、
/// キーワードと衝突したら `_` を後置する。使える文字が 1 つも無ければ `None`
fn sanitize(name: &str) -> Option<String> {
    let mut out = String::new();
    for c in name.chars() {
        if unicode_ident::is_xid_continue(c) {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let mut out = out.trim_end_matches('_').to_string();
    if out.chars().all(|c| c == '_') {
        return None;
    }
    if out
        .chars()
        .next()
        .is_some_and(|c| !unicode_ident::is_xid_start(c) && c != '_')
    {
        out.insert(0, '_');
    }
    // syn が知らない edition 2024 の予約語（gen）も避ける
    if syn::parse_str::<Ident>(&out).is_err() || out == "gen" {
        out.push('_');
    }
    Some(out)
}
