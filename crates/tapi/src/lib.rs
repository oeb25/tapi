pub mod builder;
#[cfg(feature = "endpoints")]
pub mod endpoints;
pub mod kind;
pub mod targets;

#[cfg(test)]
mod tests;

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
};

use indexmap::{IndexMap, IndexSet};
use kind::{BuiltinTypeKind, TypeKind};
pub use tapi_macro::{tapi, Tapi};

pub trait Tapi: 'static {
    fn name() -> &'static str;
    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn kind() -> TypeKind;
    fn dependencies() -> Vec<DynTapi> {
        match Self::kind() {
            TypeKind::Struct(s) => s.fields.iter().map(|f| f.ty).collect(),
            TypeKind::TupleStruct(s) => s.fields.iter().map(|f| f.ty).collect(),
            TypeKind::Enum(e) => e
                .variants
                .iter()
                .flat_map(|v| match &v.kind {
                    kind::VariantKind::Unit => Vec::new(),
                    kind::VariantKind::Tuple(fields) => fields.to_vec(),
                    kind::VariantKind::Struct(fields) => fields.iter().map(|f| f.ty).collect(),
                })
                .collect(),
            TypeKind::List(ty) => vec![ty],
            TypeKind::Option(ty) => vec![ty],
            TypeKind::Tuple(fields) => fields.to_vec(),
            TypeKind::Builtin(_) => Vec::new(),
            TypeKind::Record(k, v) => vec![k, v],
            TypeKind::Any => Vec::new(),
        }
    }
    fn path() -> Vec<&'static str> {
        let mut path = std::any::type_name::<Self>()
            .split('<')
            .next()
            .unwrap()
            .split("::")
            .collect::<Vec<_>>();
        path.pop();
        path
    }
    fn boxed() -> DynTapi
    where
        Self: Sized + 'static,
    {
        &TypedWrap::<Self>(PhantomData)
    }
    fn all_dependencies() -> Vec<DynTapi>
    where
        Self: Sized,
    {
        let mut deps = Self::dependencies();
        deps.push(Self::boxed());
        transitive_closure(deps)
    }
}

pub trait TapiDyn: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn id(&self) -> std::any::TypeId;
    fn kind(&self) -> TypeKind;
    fn dependencies(&self) -> Vec<DynTapi>;
    fn path(&self) -> Vec<&'static str>;
}

pub type DynTapi = &'static dyn TapiDyn;

impl<T: Tapi> TapiDyn for TypedWrap<T> {
    fn name(&self) -> &'static str {
        <T as Tapi>::name()
    }
    fn id(&self) -> std::any::TypeId {
        <T as Tapi>::id()
    }
    fn kind(&self) -> TypeKind {
        <T as Tapi>::kind()
    }
    fn dependencies(&self) -> Vec<DynTapi> {
        <T as Tapi>::dependencies()
    }
    fn path(&self) -> Vec<&'static str> {
        <T as Tapi>::path()
    }
}

pub struct TypedWrap<T>(PhantomData<T>);
impl<T> TypedWrap<T> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}
impl<T> Clone for TypedWrap<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}
impl<T> std::fmt::Debug for TypedWrap<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>()).finish()
    }
}

macro_rules! impl_typed {
    ($($ty:ty = $ts_name:literal & $kind:expr,)*) => {
        $(
            impl Tapi for $ty {
                fn name() -> &'static str {
                    std::any::type_name::<$ty>()
                }
                fn id() -> std::any::TypeId {
                    std::any::TypeId::of::<$ty>()
                }
                fn kind() -> TypeKind {
                    TypeKind::Builtin($kind)
                }
                fn path() -> Vec<&'static str> {
                    Vec::new()
                }
            }
        )*
    };
}
macro_rules! impl_generic {
    ($($ty:ident = $ts_name:literal & $zod_name:literal & $kind:expr,)*) => {
        $(
            impl<T: Tapi + 'static> Tapi for $ty<T> {
                fn name() -> &'static str {
                    std::any::type_name::<$ty<T>>()
                }
                fn id() -> std::any::TypeId {
                    std::any::TypeId::of::<$ty<T>>()
                }
                fn kind() -> TypeKind {
                    $kind
                }
                fn path() -> Vec<&'static str> {
                    Vec::new()
                }
            }
        )*
    };
}
impl_typed!(
    () = "unknown" & BuiltinTypeKind::Unit,
    String = "string" & BuiltinTypeKind::String,
    i8 = "number" & BuiltinTypeKind::I8,
    i16 = "number" & BuiltinTypeKind::I16,
    i32 = "number" & BuiltinTypeKind::I32,
    i64 = "number" & BuiltinTypeKind::I64,
    i128 = "number" & BuiltinTypeKind::I128,
    u8 = "number" & BuiltinTypeKind::U8,
    u16 = "number" & BuiltinTypeKind::U16,
    u32 = "number" & BuiltinTypeKind::U32,
    u64 = "number" & BuiltinTypeKind::U64,
    u128 = "number" & BuiltinTypeKind::U128,
    usize = "number" & BuiltinTypeKind::Usize,
    f32 = "number" & BuiltinTypeKind::F32,
    f64 = "number" & BuiltinTypeKind::F64,
    bool = "boolean" & BuiltinTypeKind::Bool,
    char = "string" & BuiltinTypeKind::Char,
);
#[cfg(feature = "chrono")]
impl_typed!(
    chrono::DateTime<chrono::Utc> = "string" & BuiltinTypeKind::String,
    chrono::DateTime<chrono::FixedOffset> = "string" & BuiltinTypeKind::String,
    chrono::NaiveDate = "string" & BuiltinTypeKind::String,
    chrono::NaiveTime = "string" & BuiltinTypeKind::String,
    chrono::NaiveDateTime = "string" & BuiltinTypeKind::String,
);
#[cfg(feature = "toml")]
impl_typed!(
    toml::value::Date = "string" & BuiltinTypeKind::String,
    toml::value::Datetime = "string" & BuiltinTypeKind::String,
    toml::value::Time = "string" & BuiltinTypeKind::String,
);
#[cfg(feature = "smol_str")]
impl_typed!(smol_str::SmolStr = "string" & BuiltinTypeKind::String,);
impl_generic!(
    Vec = "{}[]" & "z.array({})" & TypeKind::List(T::boxed()),
    Option = "({} | null)" & "z.optional({})" & TypeKind::Option(T::boxed()),
    HashSet = "{}[]" & "z.array({})" & TypeKind::List(T::boxed()),
    BTreeSet = "{}[]" & "z.array({})" & TypeKind::List(T::boxed()),
    IndexSet = "{}[]" & "z.array({})" & TypeKind::List(T::boxed()),
    Box = "{}" & "{}" & T::kind(),
    Rc = "{}" & "{}" & T::kind(),
    Arc = "{}" & "{}" & T::kind(),
    Cell = "{}" & "{}" & T::kind(),
    RefCell = "{}" & "{}" & T::kind(),
);
impl<const N: usize, T: Tapi + 'static> Tapi for [T; N] {
    fn name() -> &'static str {
        std::any::type_name::<[T; N]>()
    }
    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<[T; N]>()
    }
    fn kind() -> TypeKind {
        TypeKind::List(T::boxed())
    }
    fn path() -> Vec<&'static str> {
        Vec::new()
    }
}
impl<K: 'static + Tapi, V: 'static + Tapi> Tapi for HashMap<K, V> {
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn kind() -> TypeKind {
        TypeKind::Record(K::boxed(), V::boxed())
    }
    fn path() -> Vec<&'static str> {
        Vec::new()
    }
}
impl<K: 'static + Tapi, V: 'static + Tapi> Tapi for BTreeMap<K, V> {
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn kind() -> TypeKind {
        TypeKind::Record(K::boxed(), V::boxed())
    }
    fn path() -> Vec<&'static str> {
        Vec::new()
    }
}
impl<K: 'static + Tapi, V: 'static + Tapi> Tapi for IndexMap<K, V> {
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn kind() -> TypeKind {
        TypeKind::Record(K::boxed(), V::boxed())
    }
    fn path() -> Vec<&'static str> {
        Vec::new()
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),*) => {
        impl<$($ty: 'static + Tapi),*> Tapi for ($($ty,)*) {
            fn name() -> &'static str {
                std::any::type_name::<Self>()
            }
            fn id() -> std::any::TypeId {
                std::any::TypeId::of::<Self>()
            }
            fn kind() -> TypeKind {
                TypeKind::Tuple(vec![$(<$ty as Tapi>::boxed()),*])
            }
            fn path() -> Vec<&'static str> {
                Vec::new()
            }
        }
    };
}
impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);

impl Tapi for serde_json::Value {
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
    fn id() -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn kind() -> TypeKind {
        TypeKind::Any
    }
    fn path() -> Vec<&'static str> {
        Vec::new()
    }
}

fn transitive_closure(mut closure: Vec<DynTapi>) -> Vec<DynTapi> {
    let mut next = Vec::new();
    loop {
        for c in &closure {
            next.extend(c.dependencies().into_iter());
        }
        let mut done = true;
        for n in next.drain(..) {
            if closure.iter().all(|m| m.id() != n.id()) {
                done = false;
                closure.push(n);
            }
        }
        if done {
            break;
        }
    }
    closure
}
