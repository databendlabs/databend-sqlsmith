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

use crate::catalog::types::DataType;
use crate::catalog::types::NumberDataType;
use crate::catalog::types::decimal_data_type;

const FUNCTION_LIST: &str = include_str!("function_list.txt");

#[derive(Debug, Clone)]
pub(crate) struct FunctionSignature {
    pub(crate) name: String,
    pub(crate) args_type: Vec<DataType>,
    pub(crate) return_type: DataType,
}

impl FunctionSignature {
    pub(crate) fn new(
        name: impl Into<String>,
        args_type: Vec<DataType>,
        return_type: DataType,
    ) -> Self {
        Self {
            name: name.into(),
            args_type,
            return_type,
        }
    }
}

pub(crate) fn builtin_scalar_function_signatures() -> Vec<FunctionSignature> {
    FUNCTION_LIST
        .lines()
        .filter_map(parse_function_signature)
        .collect()
}

fn parse_function_signature(line: &str) -> Option<FunctionSignature> {
    let line = line.trim();
    if line.is_empty() || line.ends_with("FACTORY") {
        return None;
    }

    let (_, signature) = line.split_once(' ')?;
    let (call, return_type) = signature.split_once(" :: ")?;
    let open = call.find('(')?;
    let close = call.rfind(')')?;
    if close < open {
        return None;
    }

    let name = &call[..open];
    let args = &call[open + 1..close];
    let args_type = if args.trim().is_empty() {
        vec![]
    } else {
        split_top_level(args, ',')
            .into_iter()
            .map(|arg| parse_data_type(arg.trim()))
            .collect::<Option<Vec<_>>>()?
    };
    let return_type = parse_data_type(return_type.trim())?;

    Some(FunctionSignature::new(name, args_type, return_type))
}

fn parse_data_type(data_type: &str) -> Option<DataType> {
    let data_type = data_type.trim();
    if let Some(inner) = data_type.strip_suffix(" NULL") {
        return Some(DataType::Nullable(Box::new(parse_data_type(inner)?)));
    }

    match data_type {
        "NULL" | "Null" => return Some(DataType::Null),
        "Boolean" => return Some(DataType::Boolean),
        "Binary" => return Some(DataType::Binary),
        "String" => return Some(DataType::String),
        "UInt8" => return Some(DataType::Number(NumberDataType::UInt8)),
        "UInt16" => return Some(DataType::Number(NumberDataType::UInt16)),
        "UInt32" => return Some(DataType::Number(NumberDataType::UInt32)),
        "UInt64" => return Some(DataType::Number(NumberDataType::UInt64)),
        "Int8" => return Some(DataType::Number(NumberDataType::Int8)),
        "Int16" => return Some(DataType::Number(NumberDataType::Int16)),
        "Int32" => return Some(DataType::Number(NumberDataType::Int32)),
        "Int64" => return Some(DataType::Number(NumberDataType::Int64)),
        "Float32" => return Some(DataType::Number(NumberDataType::Float32)),
        "Float64" => return Some(DataType::Number(NumberDataType::Float64)),
        "Date" => return Some(DataType::Date),
        "Timestamp" => return Some(DataType::Timestamp),
        "TimestampTz" => return Some(DataType::TimestampTz),
        "Variant" => return Some(DataType::Variant),
        "Bitmap" => return Some(DataType::Bitmap),
        "Geometry" => return Some(DataType::Geometry),
        "Geography" => return Some(DataType::Geography),
        "Interval" => return Some(DataType::Interval),
        "Nothing" => return None,
        _ => {}
    }

    let (name, args) = parse_type_call(data_type)?;
    match name {
        "Decimal" => {
            let args = split_top_level(args, ',');
            if args.len() != 2 {
                return None;
            }
            let precision = args[0].trim().parse().ok()?;
            let scale = args[1].trim().parse().ok()?;
            Some(DataType::Decimal(decimal_data_type(precision, scale).ok()?))
        }
        "Array" => Some(DataType::Array(Box::new(parse_data_type(args)?))),
        "Map" => {
            let args = split_top_level(args, ',');
            if args.len() != 2 {
                return None;
            }
            let key_type = parse_data_type(args[0].trim())?;
            let val_type = parse_data_type(args[1].trim())?;
            Some(DataType::Map(Box::new(DataType::Tuple(vec![
                key_type, val_type,
            ]))))
        }
        "Tuple" => {
            let fields = split_top_level(args, ',')
                .into_iter()
                .map(|field| parse_data_type(field.trim()))
                .collect::<Option<Vec<_>>>()?;
            Some(DataType::Tuple(fields))
        }
        "Vector" => Some(DataType::Vector(args.trim().parse().ok()?)),
        _ => None,
    }
}

fn parse_type_call(data_type: &str) -> Option<(&str, &str)> {
    let open = data_type.find('(')?;
    let close = data_type.rfind(')')?;
    if close != data_type.len() - 1 || close < open {
        return None;
    }
    Some((&data_type[..open], &data_type[open + 1..close]))
}

fn split_top_level(input: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ch if ch == delimiter && depth == 0 => {
                parts.push(&input[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&input[start..]);
    parts
}
