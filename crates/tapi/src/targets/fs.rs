use itertools::Itertools;

use crate::{
    builder::TypesBuilder,
    kind::{BuiltinTypeKind, Field, TagType, TypeKind, VariantKind},
    DynTapi,
};

pub fn builder() -> TypesBuilder {
    TypesBuilder {
        prelude: include_str!("./prelude.fs").to_string() + "\n",
        start_namespace: Box::new(|path, name| format!("module {name} =")),
        end_namespace: Box::new(|_, _| String::new()),
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
    use BuiltinTypeKind::*;

    match ty.kind() {
        TypeKind::Struct(s) => s.attr.name.serialize_name,
        TypeKind::TupleStruct(s) => s.attr.name.serialize_name,
        TypeKind::Enum(e) => e.attr.name.serialize_name,
        TypeKind::List(ty) => format!("List<{}>", full_ty_name(ty)),
        TypeKind::Option(ty) => format!("Option<{}>", full_ty_name(ty)),
        TypeKind::Tuple(fields) => fs_tuple(&fields),
        TypeKind::Record(k, v) => format!("Map<{}, {}>", full_ty_name(k), full_ty_name(v)),
        TypeKind::Any => "any".to_string(),
        TypeKind::Builtin(b) => match b {
            U8 => "uint8",
            U16 => "uint16",
            U32 => "uint32",
            U64 => "uint64",
            U128 => "uint128",
            I8 => "int8",
            I16 => "int16",
            I32 => "int32",
            I64 => "int64",
            I128 => "int128",
            F32 => "float32",
            F64 => "float",
            Usize => "uint",
            Isize => "int",
            Bool => "bool",
            Char => "char",
            String => "string",
            Unit => "unit",
        }
        .to_string(),
    }
}

pub fn ty_decl(ty: DynTapi) -> Option<String> {
    use std::fmt::Write;
    fn inner(ty: DynTapi) -> Result<Option<String>, std::fmt::Error> {
        Ok(Some(match ty.kind() {
            TypeKind::Struct(s) => {
                let fs_fields = fs_fields(&s.fields);
                format!("type {} = {{ {fs_fields} }}", s.attr.name.serialize_name,)
            }
            TypeKind::TupleStruct(s) => {
                let fs_fields = fs_tuple(&s.fields.iter().map(|f| f.ty).collect_vec());
                format!("type {} = {fs_fields}", s.attr.name.serialize_name,)
            }
            TypeKind::Enum(e) => {
                let mut out = String::new();
                let encoding = [
                    "JsonUnionEncoding.ExternalTag",
                    "JsonUnionEncoding.UnwrapFieldlessTags",
                    "JsonUnionEncoding.UnwrapSingleFieldCases",
                ];

                let converter_options = match &e.attr.tag {
                    TagType::External => vec![format!(
                        "BaseUnionEncoding = {}",
                        encoding.iter().format(" + ")
                    )],
                    TagType::Internal { tag } => vec![
                        format!(
                            "BaseUnionEncoding = {}",
                            ["JsonUnionEncoding.UnwrapSingleFieldCases"]
                                .iter()
                                .format(" + ")
                        ),
                        format!("UnionTagName = {tag:?}"),
                    ],
                    TagType::Adjacent { tag, content } => vec![
                        format!(
                            "BaseUnionEncoding = {}",
                            ["JsonUnionEncoding.UnwrapSingleFieldCases"]
                                .iter()
                                .format(" + ")
                        ),
                        format!("UnionTagName = {tag:?}"),
                        format!("UnionFieldsName = {content:?}"),
                    ],
                    TagType::None => vec![format!(
                        "BaseUnionEncoding = {}",
                        encoding.iter().format(" + ")
                    )],
                };

                writeln!(
                    out,
                    "[<JsonFSharpConverter({})>]",
                    converter_options.iter().format(", ")
                )?;
                writeln!(out, "type {} =", e.attr.name.serialize_name)?;

                for v in &e.variants {
                    match &v.kind {
                        VariantKind::Unit => writeln!(out, "  | {}", v.name)?,
                        VariantKind::Tuple(fields) => {
                            writeln!(out, "  | {} of {}", v.name, fs_tuple(fields))?
                        }
                        VariantKind::Struct(fields) => {
                            writeln!(out, "  | {} of {}", v.name, fs_named_tuple(fields))?
                        }
                    }
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

fn fs_tuple(fields: &[DynTapi]) -> String {
    format!("{}", fields.iter().map(|f| full_ty_name(*f)).format(" * "))
}

fn fs_named_tuple(fields: &[Field]) -> String {
    format!(
        "{}",
        fields
            .iter()
            .map(|f| format!("{}: {}", f.attr.name.serialize_name, full_ty_name(f.ty)))
            .format(" * ")
    )
}

fn fs_fields(fields: &[crate::kind::Field]) -> impl std::fmt::Display + '_ {
    fields
        .iter()
        .filter(|f| !f.attr.skip_serializing)
        .map(|f| format!("{}: {}", f.name, full_ty_name(f.ty)))
        .format(" ; ")
}
