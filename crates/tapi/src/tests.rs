use serde::Serialize;

use crate::{
    targets::{fs, ts},
    Tapi,
};

#[test]
fn basic_struct() {
    // let _ = color_eyre::install();
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    struct A {
        a: i32,
        b: String,
    }

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    export type A = {
      "a": number,
      "b": string
    };
    "###);
    insta::assert_display_snapshot!(fs::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    type A =
      { a: int32
        b: string }
    "###);
}

#[test]
fn empty_struct() {
    // let _ = color_eyre::install();
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    struct A {}

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    export type A = {

    };
    "###);
    insta::assert_display_snapshot!(fs::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    type A =
      {  }
    "###);
}
#[test]
fn transparent_struct() {
    // let _ = color_eyre::install();
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    #[serde(transparent)]
    struct A {
        x: Vec<i32>,
    }

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @"export type A = number[];");
    insta::assert_display_snapshot!(fs::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    type A =
      { x: List<int32> }
    "###);
}
#[test]
fn tuple_single_struct() {
    // let _ = color_eyre::install();
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    struct A(String);

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @"export type A = string;");
    insta::assert_display_snapshot!(fs::ty_decl(A::boxed()).unwrap_or_default(), @"type A = string");
}
#[test]
fn tuple_multi_struct() {
    // let _ = color_eyre::install();
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    struct A(String, i32, Vec<A>);

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @"export type A = [string, number, tapi.tests.tuple_multi_struct.A[]];");
    insta::assert_display_snapshot!(fs::ty_decl(A::boxed()).unwrap_or_default(), @"type A = string * int32 * List<tapi.tests.tuple_multi_struct.A>");
}

#[test]
fn transparent_struct_with_multiple_fields() {
    // let _ = color_eyre::install();
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    #[serde(transparent)]
    struct A {
        #[serde(skip)]
        x: Vec<i32>,
        y: String,
    }

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @"export type A = string;");
    insta::assert_display_snapshot!(fs::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    type A =
      { y: string }
    "###);
}

#[test]
fn basic_enum() {
    // let _ = color_eyre::install();
    #[derive(Tapi)]
    #[tapi(krate = "crate")]
    enum A {
        X,
        Y,
        Z,
    }

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    export type A =
      | "X"
      | "Y"
      | "Z";
    export const A: A[] = ["X", "Y", "Z"];
    "###);
}

#[test]
fn tagged_enum() {
    // let _ = color_eyre::install();
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    #[serde(tag = "type")]
    enum A {
        X,
        Y,
        Z,
    }

    insta::assert_display_snapshot!(serde_json::to_string_pretty(&A::X).unwrap(), @r###"
    {
      "type": "X"
    }
    "###);

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    export type A =
      | { "type": "X" }
      | { "type": "Y" }
      | { "type": "Z" };
    export const A: A[] = [{ "type": "X" }, { "type": "Y" }, { "type": "Z" }];
    "###);
}

#[test]
fn tagged_enum_with_data() {
    // let _ = color_eyre::install();
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    #[serde(tag = "type")]
    enum A {
        X { wow: String },
        Y { thingy: String },
        Z,
    }

    let sample = [
        A::X {
            wow: "...".to_string(),
        },
        A::Y {
            thingy: "123".to_string(),
        },
        A::Z,
    ];
    insta::assert_display_snapshot!(serde_json::to_string_pretty(&sample).unwrap(), @r###"
    [
      {
        "type": "X",
        "wow": "..."
      },
      {
        "type": "Y",
        "thingy": "123"
      },
      {
        "type": "Z"
      }
    ]
    "###);

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    export type A =
      | { "type": "X", "wow": string }
      | { "type": "Y", "thingy": string }
      | { "type": "Z" };
    "###);
    insta::assert_display_snapshot!(fs::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    [<JsonFSharpConverter(BaseUnionEncoding = JsonUnionEncoding.UnwrapSingleFieldCases, UnionTagName = "type")>]
    type A =
      | X of wow: string
      | Y of thingy: string
      | Z
    "###);
}

#[test]
fn externally_tagged_with_data() {
    // let _ = color_eyre::install();
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    enum A {
        X(String),
        Y { thingy: String },
        Z,
        W(i32, i32),
    }
    let sample = [
        A::X("...".to_string()),
        A::Y {
            thingy: "123".to_string(),
        },
        A::Z,
        A::W(1, 2),
    ];
    insta::assert_display_snapshot!(serde_json::to_string_pretty(&sample).unwrap(), @r###"
    [
      {
        "X": "..."
      },
      {
        "Y": {
          "thingy": "123"
        }
      },
      "Z",
      {
        "W": [
          1,
          2
        ]
      }
    ]
    "###);

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    export type A =
      | { "X": string }
      | { "Y": { "thingy": string } }
      | "Z"
      | { "W": [number, number] };
    "###);
    insta::assert_display_snapshot!(fs::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    [<JsonFSharpConverter(BaseUnionEncoding = JsonUnionEncoding.ExternalTag + JsonUnionEncoding.UnwrapFieldlessTags + JsonUnionEncoding.UnwrapSingleFieldCases)>]
    type A =
      | X of string
      | Y of thingy: string
      | Z
      | W of int32 * int32
    "###);
}

#[test]
fn adjacent_with_data() {
    // let _ = color_eyre::install();
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    #[serde(tag = "type", content = "data")]
    enum A {
        X(String),
        Y { thingy: String },
        Z,
        W(i32, i32),
    }

    let sample = [
        A::X("...".to_string()),
        A::Y {
            thingy: "123".to_string(),
        },
        A::Z,
        A::W(1, 2),
    ];
    insta::assert_display_snapshot!(serde_json::to_string_pretty(&sample).unwrap(), @r###"
    [
      {
        "type": "X",
        "data": "..."
      },
      {
        "type": "Y",
        "data": {
          "thingy": "123"
        }
      },
      {
        "type": "Z"
      },
      {
        "type": "W",
        "data": [
          1,
          2
        ]
      }
    ]
    "###);

    insta::assert_display_snapshot!(ts::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    export type A =
      | { "type": "X", "data": string }
      | { "type": "Y", "data": { "thingy": string } }
      | { "type": "Z" }
      | { "type": "W", "data": [number, number] };
    "###);
    insta::assert_display_snapshot!(fs::ty_decl(A::boxed()).unwrap_or_default(), @r###"
    [<JsonFSharpConverter(BaseUnionEncoding = JsonUnionEncoding.UnwrapSingleFieldCases, UnionTagName = "type", UnionFieldsName = "data")>]
    type A =
      | X of string
      | Y of thingy: string
      | Z
      | W of int32 * int32
    "###);
}

#[test]
fn new_kind_struct() {
    // let _ = color_eyre::install();
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    struct A {
        x: String,
        y: (Box<A>, i32),
    }
    println!("{}", ts::ty_decl(A::boxed()).unwrap_or_default());
}

#[test]
fn new_kind_enum() {
    // let _ = color_eyre::install();
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    enum A {
        X(String),
        Y { thingy: String, other: Vec<A> },
        Z,
        W(i32, i32),
    }
    println!(
        "{}",
        serde_json::to_string(&A::X("...".to_string())).unwrap()
    );
    println!("{}", ts::ty_decl(A::boxed()).unwrap_or_default());
}

#[test]
fn new_kind_enum_tagged() {
    // let _ = color_eyre::install();
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    #[serde(tag = "kind")]
    enum A {
        X { wow: String },
        Y { thingy: String, other: Vec<A> },
        Z,
    }
    println!("{}", ts::ty_decl(A::boxed()).unwrap_or_default());
}

#[test]
fn new_kind_enum_tagged_and_content() {
    // let _ = color_eyre::install();
    #[derive(Tapi, Serialize)]
    #[tapi(krate = "crate")]
    #[serde(tag = "kind", content = "data")]
    enum A {
        X(String),
        Y { thingy: String, other: Vec<A> },
        Z,
        W(i32, i32),
    }
    println!("{}", ts::ty_decl(A::boxed()).unwrap_or_default());
}
