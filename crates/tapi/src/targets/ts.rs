use itertools::Itertools;

use crate::{
    builder::TypesBuilder,
    kind::{BuiltinTypeKind, TagType, TypeKind, VariantKind},
    DynTapi,
};

pub fn builder() -> TypesBuilder {
    TypesBuilder {
        prelude: include_str!("./prelude.ts").to_string() + "\n",
        start_namespace: Box::new(|_, name| format!("export namespace {} {{", name)),
        end_namespace: Box::new(|_, _| "}".to_string()),
        decl: Box::new(ty_decl),
    }
}

pub fn full_ty_name(ty: DynTapi) -> String {
    let mut name = ty_name(ty);
    for p in ty.path().iter().rev() {
        name = format!("{}.{}", p, name);
    }
    name
}

pub fn ty_name(ty: DynTapi) -> String {
    match ty.kind() {
        TypeKind::Struct(s) => s.attr.name.serialize_name,
        TypeKind::TupleStruct(s) => s.attr.name.serialize_name,
        TypeKind::Enum(e) => e.attr.name.serialize_name,
        TypeKind::List(ty) => format!("{}[]", full_ty_name(ty)),
        TypeKind::Option(ty) => format!("({} | null)", full_ty_name(ty)),
        TypeKind::Tuple(fields) => ts_tuple(&fields),
        TypeKind::Record(k, v) => format!("Record<{}, {}>", full_ty_name(k), full_ty_name(v)),
        TypeKind::Any => "any".to_string(),
        TypeKind::Builtin(b) => match b {
            BuiltinTypeKind::U8
            | BuiltinTypeKind::U16
            | BuiltinTypeKind::U32
            | BuiltinTypeKind::U64
            | BuiltinTypeKind::U128
            | BuiltinTypeKind::I8
            | BuiltinTypeKind::I16
            | BuiltinTypeKind::I32
            | BuiltinTypeKind::I64
            | BuiltinTypeKind::I128
            | BuiltinTypeKind::F32
            | BuiltinTypeKind::F64
            | BuiltinTypeKind::Usize
            | BuiltinTypeKind::Isize => "number".to_string(),
            BuiltinTypeKind::Bool => "boolean".to_string(),
            BuiltinTypeKind::Char | BuiltinTypeKind::String => "string".to_string(),
            BuiltinTypeKind::Unit => "void".to_string(),
        },
    }
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
                    let ts_fields = ts_fields(true, &s.fields);
                    format!(
                        "export type {} = {{\n{ts_fields}\n}};",
                        s.attr.name.serialize_name,
                    )
                }
            }
            TypeKind::TupleStruct(s) => {
                let ts_fields = ts_tuple(&s.fields.iter().map(|f| f.ty).collect_vec());
                format!("export type {} = {ts_fields};", s.attr.name.serialize_name,)
            }
            TypeKind::Enum(e) => {
                let mut out = String::new();
                write!(out, "export type {} =\n  | ", e.attr.name.serialize_name)?;

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
                            format!("{{ {:?}: {} }}", v.name, ts_tuple(fields))
                        }
                        TagType::Internal { tag: _ } => {
                            unreachable!("tagged tuples are not allowed by serde")
                        }
                        TagType::Adjacent { tag, content } => {
                            format!(
                                "{{ {tag:?}: {:?}, {content:?}: {} }}",
                                v.name,
                                ts_tuple(fields),
                            )
                        }
                        TagType::None => todo!("{}:{}", file!(), line!()),
                    },
                    VariantKind::Struct(fields) => match &e.attr.tag {
                        TagType::External => {
                            let ts_fields = ts_fields(false, fields);
                            format!("{{ {:?}: {{ {ts_fields} }} }}", v.name)
                        }
                        TagType::Internal { tag } => {
                            let ts_fields = ts_fields(false, fields);
                            format!("{{ {tag:?}: {:?}, {ts_fields} }}", v.name)
                        }
                        TagType::Adjacent { tag, content } => {
                            let ts_fields = ts_fields(false, fields);
                            format!(
                                "{{ {tag:?}: {:?}, {content:?}: {{ {ts_fields} }} }}",
                                v.name
                            )
                        }
                        TagType::None => todo!("TagType::None @ {}:{}", file!(), line!()),
                    },
                });

                write!(out, "{};", variants.clone().format("\n  | "))?;
                if !has_data {
                    write!(
                        out,
                        "\nexport const {}: {}[] = [{}];",
                        heck::AsShoutySnakeCase(&e.attr.name.serialize_name),
                        e.attr.name.serialize_name,
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

fn ts_tuple(fields: &[DynTapi]) -> String {
    if fields.len() == 1 {
        format!("{}", fields.iter().map(|f| full_ty_name(*f)).format(", "))
    } else {
        format!("[{}]", fields.iter().map(|f| full_ty_name(*f)).format(", "))
    }
}

fn ts_fields(multi_line: bool, fields: &[crate::kind::Field]) -> impl std::fmt::Display + '_ {
    let fields =
    fields
        .iter()
        .filter(|f| !f.attr.skip_serializing);
    if multi_line {
        fields.map(|f| format!("  {:?}: {}", f.name, full_ty_name(f.ty)))
        .join(",\n")
    } else {
        fields.map(|f| format!("{:?}: {}", f.name, full_ty_name(f.ty)))
        .join(", ")
    }
}
