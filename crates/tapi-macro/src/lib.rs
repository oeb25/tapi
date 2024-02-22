use darling::FromMeta;
use proc_macro2::Ident;
use quote::format_ident;
use serde_derive_internals::attr::TagType;
use syn::{parse_macro_input, Fields};

#[derive(Debug)]
struct Args {
    path: String,
    method: Ident,
}

impl syn::parse::Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated(input).map(
            |punctuated| {
                let mut path = None;
                let mut method = None;
                for meta in punctuated {
                    match meta {
                        syn::Meta::NameValue(syn::MetaNameValue {
                            path: syn::Path { segments, .. },
                            value,
                            ..
                        }) => {
                            let ident = segments.first().unwrap().ident.to_string();
                            match ident.as_str() {
                                "path" => {
                                    path = {
                                        match value {
                                            syn::Expr::Lit(syn::ExprLit {
                                                lit: syn::Lit::Str(lit_str),
                                                ..
                                            }) => Some(lit_str.value()),
                                            _ => panic!("unknown attribute"),
                                        }
                                    }
                                }
                                "method" => {
                                    method = {
                                        match value {
                                            syn::Expr::Path(syn::ExprPath { path, .. }) => {
                                                Some(path.segments.first().unwrap().ident.clone())
                                            }
                                            _ => panic!("unknown attribute"),
                                        }
                                    }
                                }
                                _ => panic!("unknown attribute"),
                            }
                        }
                        _ => panic!("unknown attribute"),
                    }
                }
                Args {
                    path: path.unwrap(),
                    method: method.unwrap(),
                }
            },
        )
    }
}

#[proc_macro_attribute]
pub fn tapi(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = proc_macro2::TokenStream::from(item);

    let Args { path, method } = parse_macro_input!(attr as Args);

    let fn_ = syn::parse2::<syn::ItemFn>(item.clone()).unwrap();

    let name = fn_.sig.ident;
    let mut body_ty = Vec::new();
    for inp in &fn_.sig.inputs {
        match inp {
            syn::FnArg::Receiver(_) => {
                todo!("idk what to do with receivers")
            }
            syn::FnArg::Typed(t) => {
                body_ty.push((*t.ty).clone());
            }
        }
    }
    let res_ty = match &fn_.sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => Some((**ty).clone()),
    };

    let res_ty = res_ty.unwrap_or_else(|| {
        syn::parse2::<syn::Type>(quote::quote! {
            ()
        })
        .unwrap()
    });

    let handler = match method.to_string().as_str() {
        "Get" => format_ident!("get"),
        "Post" => format_ident!("post"),
        "Put" => format_ident!("put"),
        "Delete" => format_ident!("delete"),
        "Patch" => format_ident!("patch"),
        _ => todo!("unknown method: {}", method.to_string()),
    };

    let output = quote::quote! {
        mod #name {
            #![allow(unused_parens)]

            use super::*;
            pub struct endpoint;
            impl ::tapi::endpoints::Endpoint<AppState> for endpoint {
                fn path(&self) -> &'static str {
                    #path
                }
                fn method(&self) -> ::tapi::endpoints::Method {
                    ::tapi::endpoints::Method::#method
                }
                fn bind_to(&self, router: ::axum::Router<AppState>) -> ::axum::Router<AppState> {
                    router.route(#path, ::axum::routing::#handler(super::#name))
                }
                fn body(&self) -> ::tapi::endpoints::RequestStructure {
                    let mut s = ::tapi::endpoints::RequestStructure::new(::tapi::endpoints::Method::#method);
                    #(
                        s.merge_with(
                            <#body_ty as ::tapi::endpoints::RequestTapiExtractor>::extract_request()
                        );
                    )*
                    s
                }
                fn res(&self) -> ::tapi::endpoints::ResponseTapi {
                    <#res_ty as ::tapi::endpoints::ResponseTapiExtractor>::extract_response()
                }
            }
        }

        #[tracing::instrument(name = "route", skip_all, fields(path = #path, method = stringify!(#method)))]
        #item
    };
    output.into()
}

#[derive(Debug, Default, FromMeta)]
struct DeriveInput {
    krate: Option<String>,
    path: Option<String>,
}

#[proc_macro_derive(Tapi, attributes(serde, tapi))]
pub fn tapi_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = proc_macro2::TokenStream::from(input);

    let derive_input = syn::parse2::<syn::DeriveInput>(input.clone()).unwrap();

    let tapi_derive_input = derive_input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.meta.path().is_ident("tapi") {
                Some(
                    DeriveInput::from_meta(&attr.meta)
                        .unwrap_or_else(|_| panic!("at: {}", line!())),
                )
            } else {
                None
            }
        })
        .unwrap_or_default();

    let tapi_path = tapi_derive_input
        .krate
        .as_ref()
        .map(|krate| {
            syn::parse_str(krate)
                .unwrap_or_else(|_| panic!("failed to parse krate path: {}", line!()))
        })
        .unwrap_or_else(|| quote::quote!(::tapi));

    let path = match &tapi_derive_input.path {
        Some(path) => {
            let path = path.split("::");
            quote::quote!(
                fn path() -> Vec<&'static str> {
                    vec![#(#path),*]
                }
            )
        }
        None => quote::quote!(),
    };

    let name = derive_input.ident.clone();
    let generics = derive_input.generics.params.clone();
    let mut sgenerics = Vec::new();
    let mut life_times = Vec::new();
    for g in &generics {
        match g {
            syn::GenericParam::Lifetime(l) => {
                life_times.push(l.lifetime.clone());
            }
            syn::GenericParam::Type(ty) => {
                let ident = &ty.ident;
                sgenerics.push(quote::quote!(#ident))
            }
            syn::GenericParam::Const(_) => todo!("syn::GenericParam::Const"),
        }
    }
    let serde_flags = {
        let cx = serde_derive_internals::Ctxt::new();
        let container = serde_derive_internals::ast::Container::from_ast(
            &cx,
            &derive_input,
            serde_derive_internals::Derive::Serialize,
        )
        .unwrap();
        cx.check().unwrap();
        container
    };

    let attr = build_container_attributes(&serde_flags, &tapi_path);

    let result: proc_macro2::TokenStream = match &derive_input.data {
        syn::Data::Struct(st) => {
            let mut fields = Vec::new();
            let mut kind_fields = Vec::new();
            let mut tuple_fields = Vec::new();
            for (idx, field) in st.fields.iter().enumerate() {
                let ty = field.ty.clone();
                let serde_flags = {
                    let cx = serde_derive_internals::Ctxt::new();
                    let field = serde_derive_internals::attr::Field::from_ast(
                        &cx,
                        idx,
                        field,
                        None,
                        serde_flags.attrs.default(),
                    );
                    cx.check().unwrap();
                    field
                };
                let attr = build_field_attributes(&serde_flags, &tapi_path);
                let field_name = match field.ident.clone() {
                    Some(ident) => {
                        quote::quote!(#tapi_path::kind::FieldName::Named(stringify!(#ident).to_string()))
                    }
                    None => {
                        tuple_fields.push(quote::quote!(#tapi_path::kind::TupleStructField {
                            attr: #attr,
                            ty: <#ty as #tapi_path::Tapi>::boxed(),
                        }));
                        continue;
                    }
                };
                fields.push(field.ty.clone());
                kind_fields.push(quote::quote!(
                    #tapi_path::kind::Field {
                        attr: #attr,
                        name: #field_name,
                        ty: <#ty as #tapi_path::Tapi>::boxed(),
                    }
                ));
            }
            if tuple_fields.is_empty() {
                quote::quote! {
                    #[allow(unused_parens)]
                    impl<#(#life_times,)* #(#sgenerics: 'static + #tapi_path::Tapi),*> #tapi_path::Tapi for #name<#(#life_times,)* #(#sgenerics),*> {
                        fn name() -> &'static str {
                            stringify!(#name)
                        }
                        fn id() -> std::any::TypeId {
                            std::any::TypeId::of::<#name<#(#sgenerics),*>>()
                        }
                        #path
                        fn kind() -> #tapi_path::kind::TypeKind {
                            #tapi_path::kind::TypeKind::Struct(#tapi_path::kind::Struct {
                                attr: #attr,
                                fields: [#(#kind_fields),*].to_vec(),
                            })
                        }
                    }
                }
            } else {
                assert!(kind_fields.is_empty());
                quote::quote! {
                    #[allow(unused_parens)]
                    impl<#(#life_times,)* #(#sgenerics: 'static + #tapi_path::Tapi),*> #tapi_path::Tapi for #name<#(#life_times,)* #(#sgenerics),*> {
                        fn name() -> &'static str {
                            stringify!(#name)
                        }
                        fn id() -> std::any::TypeId {
                            std::any::TypeId::of::<#name<#(#sgenerics),*>>()
                        }
                        #path
                        fn kind() -> #tapi_path::kind::TypeKind {
                            #tapi_path::kind::TypeKind::TupleStruct(#tapi_path::kind::TupleStruct {
                                attr: #attr,
                                fields: [#(#tuple_fields),*].to_vec(),
                            })
                        }
                    }
                }
            }
        }
        syn::Data::Enum(en) => {
            let mut kind_variants = Vec::new();
            for variant in &en.variants {
                let ident = &variant.ident;

                match &variant.fields {
                    Fields::Unit => {
                        kind_variants.push(quote::quote!(#tapi_path::kind::EnumVariant {
                            name: stringify!(#ident).to_string(),
                            kind: #tapi_path::kind::VariantKind::Unit,
                        }))
                    }
                    Fields::Named(fields) => {
                        let fields = fields.named.iter().map(|f| {
                            let name = f.ident.clone().expect("field did not have a name");
                            let ty = f.ty.clone();
                            let serde_flags = {
                                let cx = serde_derive_internals::Ctxt::new();
                                let field = serde_derive_internals::attr::Field::from_ast(
                                    &cx,
                                    0,
                                    f,
                                    None,
                                    serde_flags.attrs.default(),
                                );
                                cx.check().unwrap();
                                field
                            };
                            let attr = build_field_attributes(&serde_flags, &tapi_path);
                            quote::quote!(
                                #tapi_path::kind::Field {
                                    attr: #attr,
                                    name: #tapi_path::kind::FieldName::Named(stringify!(#name).to_string()),
                                    ty: <#ty as #tapi_path::Tapi>::boxed(),
                                }
                            )
                        });
                        kind_variants.push(quote::quote!(#tapi_path::kind::EnumVariant {
                            name: stringify!(#ident).to_string(),
                            kind: #tapi_path::kind::VariantKind::Struct([#(#fields),*].to_vec()),
                        }))
                    }
                    Fields::Unnamed(fields) => {
                        let fields = fields.unnamed.iter().map(|f| f.ty.clone());
                        kind_variants.push(quote::quote!(#tapi_path::kind::EnumVariant {
                            name: stringify!(#ident).to_string(),
                            kind: #tapi_path::kind::VariantKind::Tuple([#(<#fields as #tapi_path::Tapi>::boxed()),*].to_vec()),
                        }))
                    }
                }
            }
            quote::quote! {
                #[allow(unused_parens)]
                impl<#(#life_times,)* #(#sgenerics: 'static + #tapi_path::Tapi),*> #tapi_path::Tapi for #name<#(#life_times,)* #(#sgenerics),*> {
                    fn name() -> &'static str {
                        stringify!(#name)
                    }
                    fn id() -> std::any::TypeId {
                        std::any::TypeId::of::<#name>()
                    }
                    #path
                    fn kind() -> #tapi_path::kind::TypeKind {
                        #tapi_path::kind::TypeKind::Enum(#tapi_path::kind::Enum {
                            attr: #attr,
                            variants: [#(#kind_variants),*].to_vec(),
                        })
                    }
                }
            }
        }
        syn::Data::Union(_) => todo!("unions are not supported yet"),
    };

    // let pretty = prettyplease::unparse(&syn::parse2(result.clone()).unwrap());
    // eprintln!("{pretty}");
    result.into()
}

fn build_container_attributes(
    serde_flags: &serde_derive_internals::ast::Container<'_>,
    tapi_path: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let name = {
        let serialize_name = serde_flags.attrs.name().serialize_name();
        let deserialize_name = serde_flags.attrs.name().deserialize_name();
        quote::quote!(#tapi_path::kind::Name {
            serialize_name: #serialize_name.to_string(),
            deserialize_name: #deserialize_name.to_string(),
        })
    };
    let transparent = serde_flags.attrs.transparent();
    let deny_unknown_fields = serde_flags.attrs.deny_unknown_fields();
    let default = match serde_flags.attrs.default() {
        serde_derive_internals::attr::Default::None => {
            quote::quote!(#tapi_path::kind::Default::None)
        }
        serde_derive_internals::attr::Default::Default => {
            quote::quote!(#tapi_path::kind::Default::Default)
        }
        serde_derive_internals::attr::Default::Path(_) => {
            quote::quote!(#tapi_path::kind::Default::Path)
        }
    };
    let tag = {
        let tag = serde_flags.attrs.tag();
        match tag {
            TagType::External => quote::quote!(#tapi_path::kind::TagType::External),
            TagType::Internal { tag } => {
                quote::quote!(#tapi_path::kind::TagType::Internal { tag: #tag.to_string() })
            }
            TagType::Adjacent { tag, content } => quote::quote!(
                #tapi_path::kind::TagType::Adjacent {
                    tag: #tag.to_string(),
                    content: #content.to_string(),
                }
            ),
            TagType::None => quote::quote!(#tapi_path::kind::TagType::None),
        }
    };
    let type_from = match serde_flags.attrs.type_from() {
        Some(type_from) => {
            quote::quote!(Some(<#type_from as #tapi_path::Tapi>::boxed()))
        }
        None => quote::quote!(None),
    };
    let type_try_from = match serde_flags.attrs.type_try_from() {
        Some(type_try_from) => {
            quote::quote!(Some(<#type_try_from as #tapi_path::Tapi>::boxed()))
        }
        None => quote::quote!(None),
    };
    let type_into = match serde_flags.attrs.type_into() {
        Some(type_into) => {
            quote::quote!(Some(<#type_into as #tapi_path::Tapi>::boxed()))
        }
        None => quote::quote!(None),
    };
    let is_packed = serde_flags.attrs.is_packed();
    let identifier = match serde_flags.attrs.identifier() {
        serde_derive_internals::attr::Identifier::No => {
            quote::quote!(#tapi_path::kind::Identifier::No)
        }
        serde_derive_internals::attr::Identifier::Field => {
            quote::quote!(#tapi_path::kind::Identifier::Field)
        }
        serde_derive_internals::attr::Identifier::Variant => {
            quote::quote!(#tapi_path::kind::Identifier::Variant)
        }
    };
    let has_flatten = serde_flags.attrs.has_flatten();
    let non_exhaustive = serde_flags.attrs.non_exhaustive();
    quote::quote!(#tapi_path::kind::ContainerAttributes {
        name: #name,
        // rename_all_rules: todo!("rename_all_rules"),
        // rename_all_fields_rules: todo!("rename_all_fields_rules"),
        transparent: #transparent,
        deny_unknown_fields: #deny_unknown_fields,
        default: #default,
        // ser_bound: todo!("ser_bound"),
        // de_bound: todo!("de_bound"),
        tag: #tag,
        type_from: #type_from,
        type_try_from: #type_try_from,
        type_into: #type_into,
        // remote: todo!("Pa"),
        is_packed: #is_packed,
        identifier: #identifier,
        has_flatten: #has_flatten,
        // custom_serde_path: todo!("custom_serde_path"),
        // serde_path: todo!("serde_path"),
        // /// Error message generated when type can’t be deserialized. If None, default message will be used
        // expecting: todo!("expecting"),
        non_exhaustive: #non_exhaustive,
    })
}

fn build_field_attributes(
    serde_flags: &serde_derive_internals::attr::Field,
    tapi_path: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let name = {
        let serialize_name = serde_flags.name().serialize_name();
        let deserialize_name = serde_flags.name().deserialize_name();
        quote::quote!(#tapi_path::kind::Name {
            serialize_name: #serialize_name.to_string(),
            deserialize_name: #deserialize_name.to_string(),
        })
    };
    let aliases = {
        let aliases = serde_flags.aliases();
        quote::quote!([#(stringify!(#aliases).to_string()),*].into_iter().collect())
    };
    let skip_serializing = serde_flags.skip_serializing();
    let skip_deserializing = serde_flags.skip_deserializing();
    let default = match serde_flags.default() {
        serde_derive_internals::attr::Default::None => {
            quote::quote!(#tapi_path::kind::Default::None)
        }
        serde_derive_internals::attr::Default::Default => {
            quote::quote!(#tapi_path::kind::Default::Default)
        }
        serde_derive_internals::attr::Default::Path(_) => {
            quote::quote!(#tapi_path::kind::Default::Path)
        }
    };
    let flatten = serde_flags.flatten();
    let transparent = serde_flags.transparent();
    quote::quote!(#tapi_path::kind::FieldAttributes {
        name: #name,
        aliases: #aliases,
        skip_serializing: #skip_serializing,
        skip_deserializing: #skip_deserializing,
        // skip_serializing_if: #skip_serializing_if,
        default: #default,
        // serialize_with: #serialize_with,
        // deserialize_with: #deserialize_with,
        // ser_bound: #ser_bound,
        // de_bound: #de_bound,
        // borrowed_lifetimes: #borrowed_lifetimes,
        // getter: #getter,
        flatten: #flatten,
        transparent: #transparent,
    })
}
