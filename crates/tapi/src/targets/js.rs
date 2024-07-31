use itertools::Itertools;

use crate::{
    builder::TypesBuilder,
    kind::{TagType, TypeKind, VariantKind},
    DynTapi,
};

use super::ts;

pub fn builder() -> TypesBuilder {
    TypesBuilder {
        prelude: include_str!("./prelude.js").to_string() + "\n",
        // start_namespace: Box::new(|_, name| format!("export namespace {} {{", name)),
        // end_namespace: Box::new(|_, _| "}".to_string()),
        start_namespace: Box::new(|_, _| "".to_string()),
        end_namespace: Box::new(|_, _| "".to_string()),
        decl: Box::new(ty_decl),
    }
}

pub fn full_ty_name(ty: DynTapi) -> String {
    ts::full_ty_name(ty)
}

pub fn ty_name(ty: DynTapi) -> String {
    ts::ty_name(ty)
}

pub fn ty_decl(ty: DynTapi) -> Option<String> {
    use std::fmt::Write;
    fn inner(ty: DynTapi) -> Result<Option<String>, std::fmt::Error> {
        Ok(Some(match ty.kind() {
            TypeKind::Struct(s) => {
                if s.attr.transparent {
                    format!(
                        "export type {} = {};",
                        s.attr.name.serialize_name,
                        ty_name(
                            s.fields
                                .iter()
                                .find(|f| !f.attr.skip_serializing)
                                .unwrap()
                                .ty
                        ),
                    )
                } else {
                    let js_fields = js_fields(false, &s.fields);
                    format!(
                        "/**\n * @typedef {{{{ {js_fields} }}}} {} */",
                        full_ty_name(ty),
                    )
                }
            }
            TypeKind::TupleStruct(s) => {
                let js_fields = js_tuple(&s.fields.iter().map(|f| f.ty).collect_vec());
                format!("export type {} = {js_fields};", s.attr.name.serialize_name,)
            }
            TypeKind::Enum(e) => {
                let mut out = String::new();

                let has_data = e
                    .variants
                    .iter()
                    .any(|v| matches!(&v.kind, VariantKind::Tuple(_) | VariantKind::Struct(_)));

                let variants = e.variants.iter().map(|v| match &v.kind {
                    VariantKind::Unit => match &e.attr.tag {
                        TagType::External => format!("{:?}", v.name),
                        TagType::Internal { tag } | TagType::Adjacent { tag, content: _ } => {
                            format!("{{ {tag:?}: {:?} }}", v.name)
                        }
                        TagType::None => todo!("{}:{}", file!(), line!()),
                    },
                    VariantKind::Tuple(fields) => match &e.attr.tag {
                        TagType::External => {
                            format!("{{ {:?}: {} }}", v.name, js_tuple(fields))
                        }
                        TagType::Internal { tag: _ } => {
                            unreachable!("tagged tuples are not allowed by serde")
                        }
                        TagType::Adjacent { tag, content } => {
                            format!(
                                "{{ {tag:?}: {:?}, {content:?}: {} }}",
                                v.name,
                                js_tuple(fields),
                            )
                        }
                        TagType::None => todo!("{}:{}", file!(), line!()),
                    },
                    VariantKind::Struct(fields) => match &e.attr.tag {
                        TagType::External => {
                            let js_fields = js_fields(false, fields);
                            format!("{{ {:?}: {{ {js_fields} }} }}", v.name)
                        }
                        TagType::Internal { tag } => {
                            let js_fields = js_fields(false, fields);
                            format!("{{ {tag:?}: {:?}, {js_fields} }}", v.name)
                        }
                        TagType::Adjacent { tag, content } => {
                            let js_fields = js_fields(false, fields);
                            format!(
                                "{{ {tag:?}: {:?}, {content:?}: {{ {js_fields} }} }}",
                                v.name
                            )
                        }
                        TagType::None => todo!("TagType::None @ {}:{}", file!(), line!()),
                    },
                });

                write!(
                    out,
                    "/** @typedef {{{}}} {} */",
                    variants.clone().clone().format(" | "),
                    full_ty_name(ty),
                )?;

                // write!(out, "{};", variants.clone().format("\n  | "))?;
                if !has_data {
                    write!(
                        out,
                        "\nexport const {} = /** @type {{{}[]}} */ ([{}]);",
                        heck::AsShoutySnakeCase(&e.attr.name.serialize_name),
                        full_ty_name(ty),
                        variants.format(", "),
                    )?;
                }
                out
            }
            TypeKind::List(_)
            | TypeKind::Option(_)
            | TypeKind::Tuple(_)
            | TypeKind::Record(_, _)
            | TypeKind::Any
            | TypeKind::Builtin(_) => return Ok(None),
        }))
    }
    inner(ty).unwrap()
}

fn js_tuple(fields: &[DynTapi]) -> String {
    ts::ts_tuple(fields)
}

fn js_fields(multi_line: bool, fields: &[crate::kind::Field]) -> impl std::fmt::Display + '_ {
    ts::ts_fields(multi_line, fields)
}
