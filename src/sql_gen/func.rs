// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::mem;

use databend_common_ast::ast::Expr;
use databend_common_ast::ast::FunctionCall;
use databend_common_ast::ast::Identifier;
use databend_common_ast::ast::Lambda;
use databend_common_ast::ast::Literal;
use databend_common_ast::ast::OrderByExpr;
use databend_common_ast::ast::Window;
use databend_common_ast::ast::WindowDesc;
use databend_common_ast::ast::WindowFrame;
use databend_common_ast::ast::WindowFrameBound;
use databend_common_ast::ast::WindowFrameUnits;
use databend_common_ast::ast::WindowRef;
use databend_common_ast::ast::WindowSpec;
use rand::Rng;

use crate::catalog::types::ALL_FLOAT_TYPES;
use crate::catalog::types::ALL_INTEGER_TYPES;
use crate::catalog::types::DataType;
use crate::catalog::types::NumberDataType;
use crate::catalog::types::data_type_eq;
use crate::catalog::types::decimal_data_type_unchecked;
use crate::sql_gen::Column;
use crate::sql_gen::SqlGenerator;

const RANK_WINDOW_FUNCTIONS: [&str; 5] =
    ["first_value", "first", "last_value", "last", "nth_value"];

const UINT64_WINDOW_FUNCTIONS: &[&str] = &["row_number", "rank", "dense_rank", "ntile"];
const FLOAT64_WINDOW_FUNCTIONS: &[&str] = &["percent_rank", "cume_dist"];
const VALUE_WINDOW_FUNCTIONS: &[&str] = &[
    "lag",
    "lead",
    "first_value",
    "first",
    "last_value",
    "last",
    "nth_value",
];

const UINT8_AGGREGATE_FUNCTIONS: &[&str] = &["window_funnel"];
const UINT64_AGGREGATE_FUNCTIONS: &[&str] = &[
    "approx_count_distinct",
    "count",
    "sum",
    "sum0",
    "sum_zero",
    "uniq",
];
const UINT64_BITMAP_AGGREGATE_FUNCTIONS: &[&str] = &[
    "bitmap_and_count",
    "bitmap_or_count",
    "bitmap_xor_count",
    "bitmap_not_count",
];
const UINT64_BITMAP_INTERSECT_AGGREGATE_FUNCTIONS: &[&str] = &["intersect_count"];
const ARRAY_AGGREGATE_FUNCTIONS: &[&str] = &["array_agg", "list"];
const ARRAY_MOVING_AGGREGATE_FUNCTIONS: &[&str] =
    &["group_array_moving_sum", "group_array_moving_avg"];
const ARRAY_BOOLEAN_AGGREGATE_FUNCTIONS: &[&str] = &["retention"];
const ARRAY_RANGE_AGGREGATE_FUNCTIONS: &[&str] = &["range_bound"];
const ARRAY_STRING_AGGREGATE_FUNCTIONS: &[&str] = &["markov_train"];
const DECIMAL_AGGREGATE_FUNCTIONS: &[&str] = &["sum", "sum0", "sum_zero"];
const FLOAT64_UNARY_AGGREGATE_FUNCTIONS: &[&str] = &[
    "avg",
    "sum",
    "sum0",
    "sum_zero",
    "kurtosis",
    "median_tdigest",
    "median",
    "skewness",
    "stddev_pop",
    "stddev",
    "std",
    "stddev_samp",
    "quantile",
    "quantile_cont",
    "quantile_tdigest",
    "quantile_disc",
];
const FLOAT64_BINARY_AGGREGATE_FUNCTIONS: &[&str] = &[
    "covar_pop",
    "covar_samp",
    "var_pop",
    "var_samp",
    "variance_pop",
    "variance_samp",
    "quantile_tdigest_weighted",
    "median_tdigest_weighted",
];
const BITMAP_AGGREGATE_FUNCTIONS: &[&str] = &[
    "bitmap_intersect",
    "bitmap_union",
    "bitmap_or_agg",
    "bitmap_and_agg",
    "bitmap_xor_agg",
];
const BITMAP_NUMERIC_AGGREGATE_FUNCTIONS: &[&str] = &["bitmap_construct_agg", "group_bitmap"];
const STRING_AGGREGATE_FUNCTIONS: &[&str] = &["histogram", "string_agg", "listagg", "group_concat"];
const BOOLEAN_AGGREGATE_FUNCTIONS: &[&str] = &["bool_and", "bool_or"];
const VARIANT_ARRAY_AGGREGATE_FUNCTIONS: &[&str] = &["json_agg", "json_array_agg"];
const VARIANT_OBJECT_AGGREGATE_FUNCTIONS: &[&str] = &["json_object_agg"];
const GEOMETRY_AGGREGATE_FUNCTIONS: &[&str] = &[
    "st_collect",
    "st_union_agg",
    "st_intersection_agg",
    "st_envelope_agg",
];
const ANY_VALUE_AGGREGATE_FUNCTIONS: &[&str] = &["any", "any_value", "min", "max", "mode"];
const ARG_VALUE_AGGREGATE_FUNCTIONS: &[&str] = &["arg_min", "arg_max"];

impl<R: Rng> SqlGenerator<'_, R> {
    fn choose_function_name(&mut self, names: &[&str]) -> String {
        names[self.rng.gen_range(0..names.len())].to_string()
    }

    pub(crate) fn gen_scalar_func(&mut self, ty: &DataType) -> Expr {
        let mut indices = Vec::new();
        for (i, func_sig) in self.scalar_func_sigs.iter().enumerate() {
            if data_type_eq(ty, &func_sig.return_type) {
                indices.push(i);
            }
        }
        if indices.is_empty() {
            return self.gen_scalar_value(ty);
        }
        let idx = self.rng.gen_range(0..indices.len());
        let func_sig = unsafe { self.scalar_func_sigs.get_unchecked(indices[idx]) }.clone();

        self.gen_func(
            func_sig.name.clone(),
            vec![],
            func_sig.args_type,
            vec![],
            None,
            None,
        )
    }

    pub(crate) fn gen_factory_scalar_func(&mut self, ty: &DataType) -> Expr {
        let by_ty = self.rng.gen_bool(0.6);
        let (name, params, args_type) = match ty.remove_nullable() {
            DataType::String if by_ty => {
                let idx = self.rng.gen_range(0..=5);
                let name = match idx {
                    0 => "char".to_string(),
                    1 => "concat".to_string(),
                    2 => "concat_ws".to_string(),
                    3 => "regexp_replace".to_string(),
                    4 => "regexp_substr".to_string(),
                    5 => "to_string".to_string(),
                    _ => unreachable!(),
                };
                let args_type = if idx == 0 {
                    let len = self.rng.gen_range(1..=6);
                    vec![DataType::Number(NumberDataType::UInt8); len]
                } else if idx == 3 {
                    match self.rng.gen_range(3..=6) {
                        3 => vec![DataType::String; 3],
                        4 => vec![
                            DataType::String,
                            DataType::String,
                            DataType::String,
                            DataType::Number(NumberDataType::Int64),
                        ],
                        5 => vec![
                            DataType::String,
                            DataType::String,
                            DataType::String,
                            DataType::Number(NumberDataType::Int64),
                            DataType::Number(NumberDataType::Int64),
                        ],
                        6 => vec![
                            DataType::String,
                            DataType::String,
                            DataType::String,
                            DataType::Number(NumberDataType::Int64),
                            DataType::Number(NumberDataType::Int64),
                            DataType::String,
                        ],
                        _ => unreachable!(),
                    }
                } else if idx == 4 {
                    match self.rng.gen_range(2..=5) {
                        2 => vec![DataType::String; 2],
                        3 => vec![
                            DataType::String,
                            DataType::String,
                            DataType::Number(NumberDataType::Int64),
                        ],
                        4 => vec![
                            DataType::String,
                            DataType::String,
                            DataType::Number(NumberDataType::Int64),
                            DataType::Number(NumberDataType::Int64),
                        ],
                        5 => vec![
                            DataType::String,
                            DataType::String,
                            DataType::Number(NumberDataType::Int64),
                            DataType::Number(NumberDataType::Int64),
                            DataType::String,
                        ],
                        _ => unreachable!(),
                    }
                } else if idx == 5 {
                    if self.rng.gen_bool(0.5) {
                        vec![DataType::Decimal(decimal_data_type_unchecked(20, 0)); 1]
                    } else {
                        vec![DataType::Decimal(decimal_data_type_unchecked(39, 0)); 1]
                    }
                } else {
                    let len = self.rng.gen_range(2..=6);
                    vec![DataType::String; len]
                };
                (name, vec![], args_type)
            }
            DataType::Boolean if by_ty => {
                let idx = self.rng.gen_range(0..=3);
                let name = match idx {
                    0 => "and_filters".to_string(),
                    1 => "regexp_like".to_string(),
                    2 => {
                        let comp_func = ["eq", "gt", "gte", "lt", "lte", "noteq"];
                        comp_func[self.rng.gen_range(0..=5)].to_string()
                    }
                    3 => "ignore".to_string(),

                    _ => unreachable!(),
                };
                let args_type = match idx {
                    0 => vec![DataType::Boolean; 2],
                    1 => match self.rng.gen_range(2..=3) {
                        2 => vec![DataType::String; 2],
                        3 => vec![DataType::String; 3],
                        _ => unreachable!(),
                    },
                    2 => {
                        let ty = self.gen_data_type();
                        vec![ty; 2]
                    }
                    3 => {
                        let ty1 = self.gen_data_type();
                        let ty2 = self.gen_data_type();
                        let ty3 = self.gen_data_type();
                        vec![ty1, ty2, ty3]
                    }
                    _ => unreachable!(),
                };
                (name, vec![], args_type)
            }
            DataType::Number(_) if by_ty => {
                let idx = self.rng.gen_range(0..=4);
                let name = match idx {
                    0 => "point_in_ellipses".to_string(),
                    1 => "point_in_polygon".to_string(),
                    2 => "regexp_instr".to_string(),
                    3 => {
                        let arithmetic_func = ["plus", "minus", "multiply", "divide"];
                        arithmetic_func[self.rng.gen_range(0..=3)].to_string()
                    }
                    4 => {
                        let array_func = [
                            "array_approx_count_distinct",
                            "array_avg",
                            "array_kurtosis",
                            "array_median",
                            "array_skewness",
                            "array_std",
                            "array_stddev",
                            "array_stddev_pop",
                            "array_stddev_samp",
                            "array_sum",
                        ];
                        array_func[self.rng.gen_range(0..=9)].to_string()
                    }
                    _ => unreachable!(),
                };

                let args_type = match idx {
                    0 => vec![DataType::Number(NumberDataType::Float64); 7],
                    1 => {
                        let mut args_type = vec![];
                        let arg1 =
                            DataType::Tuple(vec![DataType::Number(NumberDataType::Float64); 3]);
                        let arg2 =
                            DataType::Array(Box::from(DataType::Number(NumberDataType::Float64)));
                        let arg3 =
                            DataType::Array(Box::from(DataType::Number(NumberDataType::Int64)));
                        args_type.push(arg1);
                        args_type.push(arg2);
                        args_type.push(arg3);
                        args_type
                    }
                    2 => match self.rng.gen_range(2..=6) {
                        2 => vec![DataType::String; 2],
                        3 => vec![
                            DataType::String,
                            DataType::String,
                            DataType::Number(NumberDataType::Int64),
                        ],
                        4 => vec![
                            DataType::String,
                            DataType::String,
                            DataType::Number(NumberDataType::Int64),
                            DataType::Number(NumberDataType::Int64),
                        ],
                        5 => vec![
                            DataType::String,
                            DataType::String,
                            DataType::Number(NumberDataType::Int64),
                            DataType::Number(NumberDataType::Int64),
                            DataType::Number(NumberDataType::Int64),
                        ],
                        6 => vec![
                            DataType::String,
                            DataType::String,
                            DataType::Number(NumberDataType::Int64),
                            DataType::Number(NumberDataType::Int64),
                            DataType::Number(NumberDataType::Int64),
                            DataType::String,
                        ],
                        _ => unreachable!(),
                    },
                    3 => {
                        let mut args_type = vec![];
                        let int_num = ALL_INTEGER_TYPES.len();
                        let float_num = ALL_FLOAT_TYPES.len();
                        let left = ALL_INTEGER_TYPES[self.rng.gen_range(0..=int_num - 1)];
                        let right = ALL_FLOAT_TYPES[self.rng.gen_range(0..=float_num - 1)];
                        if self.rng.gen_bool(0.5) {
                            args_type.push(DataType::Number(left));
                            args_type.push(DataType::Number(right));
                        } else {
                            args_type.push(DataType::Number(right));
                            args_type.push(DataType::Number(left));
                        }
                        args_type
                    }
                    4 => {
                        let inner_ty = self.gen_number_data_type();
                        vec![DataType::Array(Box::new(inner_ty))]
                    }
                    _ => unreachable!(),
                };

                (name, vec![], args_type)
            }
            DataType::Array(box inner_ty) if by_ty => {
                let name = "array".to_string();
                let len = self.rng.gen_range(0..=4);
                let args_type = vec![inner_ty; len];
                (name, vec![], args_type)
            }
            DataType::Map(box DataType::Tuple(inner_tys)) if by_ty => {
                let key_ty = inner_tys[0].clone();
                let name = if self.rng.gen_bool(0.5) {
                    "map_delete".to_string()
                } else {
                    "map_pick".to_string()
                };
                if self.rng.gen_bool(0.5) {
                    let len = self.rng.gen_range(1..=5);
                    let args_type = vec![key_ty; len];
                    (name, vec![], args_type)
                } else {
                    (name, vec![], vec![DataType::Array(Box::new(key_ty))])
                }
            }
            DataType::Decimal(_) if by_ty => {
                let decimal = ["to_float64", "to_float32", "to_decimal", "try_to_decimal"];
                let name = decimal[self.rng.gen_range(0..=3)].to_string();
                if name == "to_decimal" || name == "try_to_decimal" {
                    let args_type = vec![self.gen_data_type(); 1];
                    let params = vec![Literal::UInt64(20), Literal::UInt64(19)];
                    (name, params, args_type)
                } else {
                    let ty = if self.rng.gen_bool(0.5) {
                        DataType::Decimal(decimal_data_type_unchecked(28, 0))
                    } else {
                        DataType::Decimal(decimal_data_type_unchecked(39, 0))
                    };
                    let args_type = vec![ty; 1];
                    let params = vec![];
                    (name, params, args_type)
                }
            }
            DataType::Tuple(inner_tys) if by_ty => {
                let name = "tuple".to_string();
                (name, vec![], inner_tys)
            }
            DataType::Variant if by_ty => {
                if self.rng.gen_bool(0.5) {
                    let json_func = ["json_array", "json_object", "json_object_keep_null"];
                    let name = json_func[self.rng.gen_range(0..=2)].to_string();
                    let len = self.rng.gen_range(0..=2);
                    let mut args_type = Vec::with_capacity(len * 2);
                    for _ in 0..len {
                        args_type.push(DataType::String);
                        args_type.push(self.gen_data_type());
                    }
                    (name, vec![], args_type)
                } else {
                    let json_func = ["unnest", "json_path_query"];
                    let name = json_func[self.rng.gen_range(0..=1)].to_string();
                    let args_type = vec![ty.clone()];
                    (name, vec![], args_type)
                }
            }
            _ => {
                if self.rng.gen_bool(0.3) {
                    let name = "if".to_string();
                    let len = self.rng.gen_range(1..=3) * 2 + 1;
                    let mut args_type = Vec::with_capacity(len);
                    for i in 0..len {
                        if i % 2 == 0 && i != len - 1 {
                            args_type.push(DataType::Boolean);
                        } else {
                            args_type.push(ty.clone());
                        }
                    }
                    (name, vec![], args_type)
                } else {
                    let array_func = [
                        "unnest",
                        "array_any",
                        "array_count",
                        "array_max",
                        "array_min",
                    ];
                    let name = array_func[self.rng.gen_range(0..=4)].to_string();
                    let args_type = vec![DataType::Array(Box::new(ty.clone()))];
                    (name, vec![], args_type)
                }
            }
        };

        self.gen_func(name, params, args_type, vec![], None, None)
    }

    pub(crate) fn gen_agg_func(&mut self, ty: &DataType) -> Expr {
        let by_ty = self.rng.gen_bool(0.6);
        let (name, params, mut args_type) = match ty.remove_nullable() {
            DataType::Number(NumberDataType::UInt8) if by_ty => {
                let name = self.choose_function_name(UINT8_AGGREGATE_FUNCTIONS);
                let other_type = vec![DataType::Boolean; 6];
                let mut args_type = Vec::with_capacity(7);

                match self.rng.gen_range(0..=2) {
                    0 => args_type.push(self.gen_number_data_type()),
                    1 => args_type.push(DataType::Date),
                    2 => args_type.push(DataType::Timestamp),
                    _ => unreachable!(),
                };
                args_type.extend_from_slice(&other_type);
                let params = vec![Literal::UInt64(self.rng.gen_range(1..=10))];
                (name, params, args_type)
            }
            DataType::Number(NumberDataType::UInt64) if by_ty => {
                let (name, args_type, params) = match self.rng.gen_range(0..=2) {
                    0 => {
                        let name = self.choose_function_name(UINT64_AGGREGATE_FUNCTIONS);
                        let args_type = if name == "sum" {
                            vec![self.gen_all_number_data_type()]
                        } else {
                            vec![self.gen_data_type()]
                        };
                        (name, args_type, vec![])
                    }
                    1 => {
                        let name = self.choose_function_name(UINT64_BITMAP_AGGREGATE_FUNCTIONS);
                        let args_type = if self.rng.gen_bool(0.5) {
                            vec![DataType::Bitmap]
                        } else {
                            vec![DataType::Nullable(Box::new(DataType::Bitmap))]
                        };
                        (name, args_type, vec![])
                    }
                    2 => {
                        let name =
                            self.choose_function_name(UINT64_BITMAP_INTERSECT_AGGREGATE_FUNCTIONS);
                        let args_type = if self.rng.gen_bool(0.5) {
                            vec![DataType::Bitmap; 2]
                        } else {
                            vec![DataType::Nullable(Box::new(DataType::Bitmap)); 2]
                        };
                        let params = vec![
                            Literal::UInt64(self.rng.gen_range(1..=10)),
                            Literal::UInt64(self.rng.gen_range(1..=10)),
                        ];
                        (name, args_type, params)
                    }
                    _ => unreachable!(),
                };
                (name, params, args_type)
            }
            DataType::Array(_) if by_ty => {
                let (name, args_type, params) = match self.rng.gen_range(0..=4) {
                    0 => (
                        self.choose_function_name(ARRAY_AGGREGATE_FUNCTIONS),
                        vec![self.gen_data_type()],
                        vec![],
                    ),
                    1 => {
                        let args_type = if self.rng.gen_bool(0.9) {
                            vec![DataType::Boolean; 6]
                        } else {
                            vec![self.gen_data_type(); 6]
                        };
                        (
                            self.choose_function_name(ARRAY_BOOLEAN_AGGREGATE_FUNCTIONS),
                            args_type,
                            vec![],
                        )
                    }
                    2 => {
                        let params = if self.rng.gen_bool(0.5) {
                            vec![Literal::UInt64(self.rng.gen_range(1..=3))]
                        } else {
                            vec![]
                        };
                        (
                            self.choose_function_name(ARRAY_MOVING_AGGREGATE_FUNCTIONS),
                            vec![self.gen_all_number_data_type()],
                            params,
                        )
                    }
                    3 => (
                        self.choose_function_name(ARRAY_RANGE_AGGREGATE_FUNCTIONS),
                        vec![self.gen_simple_common_data_type()],
                        vec![Literal::UInt64(self.rng.gen_range(2..=10))],
                    ),
                    4 => (
                        self.choose_function_name(ARRAY_STRING_AGGREGATE_FUNCTIONS),
                        vec![DataType::String],
                        vec![],
                    ),
                    _ => unreachable!(),
                };
                (name, params, args_type)
            }
            DataType::Decimal(_) if by_ty => {
                let name = self.choose_function_name(DECIMAL_AGGREGATE_FUNCTIONS);
                let params = vec![];
                let args_type = vec![self.gen_decimal_data_type()];
                (name, params, args_type)
            }
            DataType::Number(NumberDataType::Float64) if by_ty => {
                let binary = self.rng.gen_bool(0.35);
                let name = if binary {
                    self.choose_function_name(FLOAT64_BINARY_AGGREGATE_FUNCTIONS)
                } else {
                    self.choose_function_name(FLOAT64_UNARY_AGGREGATE_FUNCTIONS)
                };
                let args_type = if binary {
                    let weight_ty = if name == "quantile_tdigest_weighted"
                        || name == "median_tdigest_weighted"
                    {
                        DataType::Number(NumberDataType::UInt64)
                    } else {
                        self.gen_all_number_data_type()
                    };
                    vec![self.gen_all_number_data_type(), weight_ty]
                } else {
                    vec![self.gen_all_number_data_type()]
                };

                let params = if name.starts_with("quantile") {
                    if self.rng.gen_bool(0.5) {
                        vec![Literal::Float64(self.rng.gen_range(0.01..=0.99))]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };
                (name, params, args_type)
            }
            DataType::Bitmap if by_ty => {
                let numeric_arg = self.rng.gen_bool(0.3);
                let name = if numeric_arg {
                    self.choose_function_name(BITMAP_NUMERIC_AGGREGATE_FUNCTIONS)
                } else {
                    self.choose_function_name(BITMAP_AGGREGATE_FUNCTIONS)
                };
                let params = vec![];
                let args_type = if numeric_arg {
                    vec![DataType::Number(NumberDataType::UInt64)]
                } else {
                    vec![DataType::Bitmap]
                };
                (name, params, args_type)
            }
            DataType::String if by_ty => {
                let name = self.choose_function_name(STRING_AGGREGATE_FUNCTIONS);
                if name == "histogram" {
                    let arg_type = self.gen_simple_common_data_type();
                    (name, vec![], vec![arg_type])
                } else {
                    let args_type = if self.rng.gen_bool(0.6) {
                        vec![DataType::String]
                    } else {
                        vec![DataType::String; 2]
                    };
                    let params = vec![];
                    (name, params, args_type)
                }
            }
            DataType::Boolean if by_ty => {
                let name = self.choose_function_name(BOOLEAN_AGGREGATE_FUNCTIONS);
                (name, vec![], vec![DataType::Boolean])
            }
            DataType::Variant if by_ty => {
                if self.rng.gen_bool(0.5) {
                    let name = self.choose_function_name(VARIANT_ARRAY_AGGREGATE_FUNCTIONS);
                    let arg_type = self.gen_simple_data_type();
                    (name, vec![], vec![arg_type])
                } else {
                    let name = self.choose_function_name(VARIANT_OBJECT_AGGREGATE_FUNCTIONS);
                    let key_type = DataType::String;
                    let val_type = self.gen_simple_data_type();
                    (name, vec![], vec![key_type, val_type])
                }
            }
            DataType::Geometry if by_ty => {
                let name = self.choose_function_name(GEOMETRY_AGGREGATE_FUNCTIONS);
                let arg_type = DataType::Geometry;
                (name, vec![], vec![arg_type])
            }
            _ => {
                let arg_value = self.rng.gen_bool(0.3);
                let name = if arg_value {
                    self.choose_function_name(ARG_VALUE_AGGREGATE_FUNCTIONS)
                } else {
                    self.choose_function_name(ANY_VALUE_AGGREGATE_FUNCTIONS)
                };
                let params = vec![];
                let args_type = if arg_value {
                    vec![ty.clone(), self.gen_simple_data_type()]
                } else {
                    vec![ty.clone()]
                };
                (name, params, args_type)
            }
        };
        // test combinator, only need test _if and _distinct
        let idx = self.rng.gen_range(0..=2);
        let (name, params, args_type) = match idx {
            0 => (name, params, args_type),
            1 => {
                let name = name + "_if";
                args_type.push(DataType::Boolean);
                (name, params, args_type)
            }
            2 => {
                let name = name + "_distinct";
                (name, params, args_type)
            }
            _ => unreachable!(),
        };

        let window = if self.rng.gen_bool(0.8) {
            None
        } else {
            self.gen_window(&name)
        };

        self.gen_func(name, params, args_type, vec![], window, None)
    }

    pub(crate) fn gen_window_func(&mut self, ty: &DataType) -> Expr {
        let by_ty = self.rng.gen_bool(0.6);
        let ty = ty.clone();
        match ty {
            DataType::Number(NumberDataType::UInt64) if by_ty => {
                let name =
                    UINT64_WINDOW_FUNCTIONS[self.rng.gen_range(0..UINT64_WINDOW_FUNCTIONS.len())];
                let args_type = if name == "ntile" {
                    vec![DataType::Number(NumberDataType::UInt64)]
                } else {
                    vec![]
                };
                let window = self.gen_window(name);
                self.gen_func(name.to_string(), vec![], args_type, vec![], window, None)
            }
            DataType::Number(NumberDataType::Float64) if by_ty => {
                let name =
                    FLOAT64_WINDOW_FUNCTIONS[self.rng.gen_range(0..FLOAT64_WINDOW_FUNCTIONS.len())];
                let window = self.gen_window(name);
                self.gen_func(name.to_string(), vec![], vec![], vec![], window, None)
            }
            _ => {
                let name =
                    VALUE_WINDOW_FUNCTIONS[self.rng.gen_range(0..VALUE_WINDOW_FUNCTIONS.len())];
                let args_type = if name == "lag" || name == "lead" {
                    vec![ty; 3]
                } else if name == "nth_value" {
                    vec![ty, DataType::Number(NumberDataType::UInt64)]
                } else {
                    vec![ty]
                };
                let window = self.gen_window(name);
                self.gen_func(name.to_string(), vec![], args_type, vec![], window, None)
            }
        }
    }

    fn gen_window(&mut self, func_name: &str) -> Option<WindowDesc> {
        let ignore_nulls = if RANK_WINDOW_FUNCTIONS.contains(&func_name) {
            Some(self.rng.gen_bool(0.2))
        } else {
            None
        };
        if self.rng.gen_bool(0.2) && !self.windows_name.is_empty() {
            let len = self.windows_name.len();
            let name = if len == 1 {
                self.windows_name[0].to_string()
            } else {
                self.windows_name[self.rng.gen_range(0..=len - 1)].to_string()
            };

            Some(WindowDesc {
                ignore_nulls,
                window: Window::WindowReference(WindowRef {
                    window_name: Identifier::from_name(None, name),
                }),
            })
        } else {
            let window_spec = self.gen_window_spec();
            Some(WindowDesc {
                ignore_nulls,
                window: Window::WindowSpec(window_spec),
            })
        }
    }

    pub(crate) fn gen_window_spec(&mut self) -> WindowSpec {
        let ty = self.gen_data_type();
        let expr1 = self.gen_scalar_value(&ty);
        let expr2 = self.gen_scalar_value(&ty);
        let expr3 = self.gen_scalar_value(&ty);
        let expr4 = self.gen_scalar_value(&ty);

        let order_by = vec![
            OrderByExpr {
                expr: expr1,
                asc: None,
                nulls_first: None,
            },
            OrderByExpr {
                expr: expr2,
                asc: Some(true),
                nulls_first: Some(true),
            },
        ];
        WindowSpec {
            existing_window_name: None,
            partition_by: vec![expr3, expr4],
            order_by,
            window_frame: if self.rng.gen_bool(0.8) {
                None
            } else {
                Some(WindowFrame {
                    units: WindowFrameUnits::Rows,
                    start_bound: WindowFrameBound::Preceding(None),
                    end_bound: WindowFrameBound::CurrentRow,
                })
            },
        }
    }

    pub(crate) fn gen_lambda_func(&mut self, ty: &DataType) -> Expr {
        // return value of lambda function must be an array type
        if !matches!(ty, &DataType::Array(_)) {
            return self.gen_simple_expr(ty);
        }
        let inner_ty = ty.as_array().unwrap();

        let current_cte_tables = mem::take(&mut self.cte_tables);
        let current_bound_tables = mem::take(&mut self.bound_tables);
        let current_bound_columns = mem::take(&mut self.bound_columns);
        let current_is_join = self.is_join;

        self.cte_tables = vec![];
        self.bound_tables = vec![];
        self.bound_columns = vec![];
        self.is_join = false;

        let name = if data_type_eq(&inner_ty.remove_nullable(), &DataType::Boolean) {
            "array_filter".to_string()
        } else if self.rng.gen_bool(0.5) {
            "array_transform".to_string()
        } else {
            "array_apply".to_string()
        };

        let args_type = vec![ty.clone()];
        let lambda_name = format!("l{}", self.gen_random_name());
        let lambda_column = Column::new(None, lambda_name.clone(), 0, ty.clone());
        self.bound_columns.push(lambda_column);

        let lambda_expr = self.gen_expr(inner_ty);

        let lambda = Lambda {
            params: vec![Identifier::from_name(None, lambda_name)],
            expr: Box::new(lambda_expr),
        };

        self.cte_tables = current_cte_tables;
        self.bound_tables = current_bound_tables;
        self.bound_columns = current_bound_columns;
        self.is_join = current_is_join;

        self.gen_func(name, vec![], args_type, vec![], None, Some(lambda))
    }

    fn gen_func(
        &mut self,
        name: String,
        params: Vec<Literal>,
        args_type: Vec<DataType>,
        order_by: Vec<OrderByExpr>,
        window: Option<WindowDesc>,
        lambda: Option<Lambda>,
    ) -> Expr {
        let distinct = if name == *"count" {
            self.rng.gen_bool(0.5)
        } else {
            false
        };

        let mut args = vec![];
        for (i, ty) in args_type.iter().enumerate() {
            if name == *"lead" || name == *"lag" || name == *"nth_value" {
                if i == 1 {
                    args.push(Expr::Literal {
                        span: None,
                        value: Literal::UInt64(self.rng.gen_range(1..=10)),
                    })
                } else {
                    args.push(self.gen_expr(ty))
                }
            } else if name == "factorial" {
                args.push(Expr::Literal {
                    span: None,
                    value: Literal::UInt64(self.rng.gen_range(0..=20)),
                })
            } else {
                args.push(self.gen_expr(ty))
            }
        }

        let params = params
            .into_iter()
            .map(|param| Expr::Literal {
                span: None,
                value: param,
            })
            .collect();

        let name = Identifier::from_name(None, name);
        Expr::FunctionCall {
            span: None,
            func: FunctionCall {
                distinct,
                name,
                args,
                params,
                order_by,
                window,
                lambda,
            },
        }
    }
}
