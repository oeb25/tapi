use std::collections::BTreeSet;

use crate::DynTapi;

#[derive(Debug, Clone)]
pub struct ContainerAttributes {
    pub name: Name,
    // pub rename_all_rules: RenameAllRules,
    // pub rename_all_fields_rules: RenameAllRules,
    pub transparent: bool,
    pub deny_unknown_fields: bool,
    pub default: Default,
    // pub ser_bound: Option<Vec<WherePredicate>>,
    // pub de_bound: Option<Vec<WherePredicate>>,
    pub tag: TagType,
    pub type_from: Option<DynTapi>,
    pub type_try_from: Option<DynTapi>,
    pub type_into: Option<DynTapi>,
    // pub remote: Option<Path>,
    pub is_packed: bool,
    pub identifier: Identifier,
    pub has_flatten: bool,
    // pub custom_serde_path: Option<Path>,
    // pub serde_path: Cow<'_, Path>,
    // /// Error message generated when type can’t be deserialized. If None, default message will be used
    // pub expecting: Option<String>,
    pub non_exhaustive: bool,
}

#[derive(Debug, Clone)]
pub struct Name {
    pub serialize_name: String,
    pub deserialize_name: String,
}

#[derive(Debug, Clone)]
pub enum Default {
    None,
    Default,
    Path,
}

#[derive(Debug, Clone)]
pub enum TagType {
    External,
    Internal { tag: String },
    Adjacent { tag: String, content: String },
    None,
}

#[derive(Debug, Clone)]
pub enum Identifier {
    No,
    Field,
    Variant,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct(Struct),
    TupleStruct(TupleStruct),
    Enum(Enum),
    List(DynTapi),
    Option(DynTapi),
    Tuple(Vec<DynTapi>),
    Builtin(BuiltinTypeKind),
    Record(DynTapi, DynTapi),
    Any,
}

#[derive(Debug, Clone)]
pub enum BuiltinTypeKind {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Usize,
    Isize,
    Bool,
    Char,
    String,
    Unit,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub attr: ContainerAttributes,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone)]
pub struct TupleStruct {
    pub attr: ContainerAttributes,
    pub fields: Vec<TupleStructField>,
}

#[derive(Debug, Clone)]
pub struct TupleStructField {
    pub attr: FieldAttributes,
    pub ty: DynTapi,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub attr: FieldAttributes,
    pub name: FieldName,
    pub ty: DynTapi,
}

#[derive(Clone)]
pub enum FieldName {
    Named(Name),
    Index(usize),
}

impl std::fmt::Debug for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldName::Named(name) => write!(f, "{:?}", name),
            FieldName::Index(idx) => write!(f, "{:?}", idx),
        }
    }
}

// impl std::fmt::Display for FieldName {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             FieldName::Named(name) => write!(f, "{}", name),
//             FieldName::Index(idx) => write!(f, "{}", idx),
//         }
//     }
// }

#[derive(Debug, Clone)]
pub struct FieldAttributes {
    pub name: Name,
    pub aliases: BTreeSet<String>,
    pub skip_serializing: bool,
    pub skip_deserializing: bool,
    // pub skip_serializing_if: Option<ExprPath>,
    pub default: Default,
    // pub serialize_with: Option<ExprPath>,
    // pub deserialize_with: Option<ExprPath>,
    // pub ser_bound: Option<Vec<WherePredicate>>,
    // pub de_bound: Option<Vec<WherePredicate>>,
    // pub borrowed_lifetimes: BTreeSet<Lifetime>,
    // pub getter: Option<ExprPath>,
    pub flatten: bool,
    pub transparent: bool,
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub attr: ContainerAttributes,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub kind: VariantKind,
}

#[derive(Debug, Clone)]
pub enum VariantKind {
    Unit,
    Tuple(Vec<DynTapi>),
    Struct(Vec<Field>),
}
