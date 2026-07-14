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

use databend_common_ast::ast::TypeName;

use crate::error::Result;
use crate::error::SqlsmithError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumberDataType {
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
}

pub(crate) const ALL_INTEGER_TYPES: &[NumberDataType] = &[
    NumberDataType::UInt8,
    NumberDataType::UInt16,
    NumberDataType::UInt32,
    NumberDataType::UInt64,
    NumberDataType::Int8,
    NumberDataType::Int16,
    NumberDataType::Int32,
    NumberDataType::Int64,
];

pub(crate) const ALL_FLOAT_TYPES: &[NumberDataType] =
    &[NumberDataType::Float32, NumberDataType::Float64];

const MAX_DECIMAL_PRECISION: u8 = 76;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DecimalSize {
    pub(crate) precision: u8,
    pub(crate) scale: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecimalDataType {
    Decimal128(DecimalSize),
    Decimal256(DecimalSize),
}

impl DecimalDataType {
    pub(crate) fn decimal_size(&self) -> &DecimalSize {
        match self {
            DecimalDataType::Decimal128(size) | DecimalDataType::Decimal256(size) => size,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DataType {
    Null,
    EmptyArray,
    EmptyMap,
    Boolean,
    Binary,
    String,
    Number(NumberDataType),
    Decimal(DecimalDataType),
    Timestamp,
    TimestampTz,
    Date,
    Nullable(Box<DataType>),
    Array(Box<DataType>),
    Map(Box<DataType>),
    Tuple(Vec<DataType>),
    Variant,
    Bitmap,
    Geometry,
    Geography,
    Interval,
    Vector(u64),
}

impl DataType {
    pub(crate) fn wrap_nullable(&self) -> DataType {
        match self {
            DataType::Null | DataType::Nullable(_) => self.clone(),
            _ => DataType::Nullable(Box::new(self.clone())),
        }
    }

    pub(crate) fn is_nullable_or_null(&self) -> bool {
        matches!(self, DataType::Nullable(_) | DataType::Null)
    }

    pub(crate) fn remove_nullable(&self) -> DataType {
        match self {
            DataType::Nullable(inner) => (**inner).clone(),
            _ => self.clone(),
        }
    }

    pub(crate) fn as_array(&self) -> Option<&DataType> {
        match self {
            DataType::Array(inner) => Some(inner),
            _ => None,
        }
    }
}

pub(crate) fn data_type_eq(left: &DataType, right: &DataType) -> bool {
    match (left, right) {
        (DataType::Null, DataType::Null)
        | (DataType::EmptyArray, DataType::EmptyArray)
        | (DataType::EmptyMap, DataType::EmptyMap)
        | (DataType::Boolean, DataType::Boolean)
        | (DataType::Binary, DataType::Binary)
        | (DataType::String, DataType::String)
        | (DataType::Timestamp, DataType::Timestamp)
        | (DataType::TimestampTz, DataType::TimestampTz)
        | (DataType::Date, DataType::Date)
        | (DataType::Bitmap, DataType::Bitmap)
        | (DataType::Variant, DataType::Variant)
        | (DataType::Geometry, DataType::Geometry)
        | (DataType::Geography, DataType::Geography)
        | (DataType::Interval, DataType::Interval) => true,
        (DataType::Number(left), DataType::Number(right)) => left == right,
        (DataType::Decimal(left), DataType::Decimal(right)) => left == right,
        (DataType::Nullable(left), DataType::Nullable(right))
        | (DataType::Array(left), DataType::Array(right))
        | (DataType::Map(left), DataType::Map(right)) => data_type_eq(left, right),
        (DataType::Tuple(left), DataType::Tuple(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| data_type_eq(left, right))
        }
        (DataType::Vector(left), DataType::Vector(right)) => left == right,
        _ => false,
    }
}

pub(crate) fn decimal_size(precision: u8, scale: u8) -> Result<DecimalSize> {
    if precision == 0 || precision > MAX_DECIMAL_PRECISION {
        return Err(SqlsmithError::BadArguments(format!(
            "decimal precision must be between 1 and {}, got {}",
            MAX_DECIMAL_PRECISION, precision
        )));
    }
    if scale > precision {
        return Err(SqlsmithError::BadArguments(format!(
            "decimal scale {} exceeds precision {}",
            scale, precision
        )));
    }
    Ok(DecimalSize { precision, scale })
}

pub(crate) fn decimal_data_type(precision: u8, scale: u8) -> Result<DecimalDataType> {
    Ok(decimal_data_type_from_size(decimal_size(precision, scale)?))
}

pub(crate) fn decimal_data_type_unchecked(precision: u8, scale: u8) -> DecimalDataType {
    decimal_data_type_from_size(DecimalSize { precision, scale })
}

fn decimal_data_type_from_size(size: DecimalSize) -> DecimalDataType {
    if size.precision <= 38 {
        DecimalDataType::Decimal128(size)
    } else {
        DecimalDataType::Decimal256(size)
    }
}

pub(crate) fn resolve_type_name(type_name: &TypeName, not_null: bool) -> Result<DataType> {
    let data_type = match type_name {
        TypeName::Boolean => DataType::Boolean,
        TypeName::UInt8 => DataType::Number(NumberDataType::UInt8),
        TypeName::UInt16 => DataType::Number(NumberDataType::UInt16),
        TypeName::UInt32 => DataType::Number(NumberDataType::UInt32),
        TypeName::UInt64 => DataType::Number(NumberDataType::UInt64),
        TypeName::Int8 => DataType::Number(NumberDataType::Int8),
        TypeName::Int16 => DataType::Number(NumberDataType::Int16),
        TypeName::Int32 => DataType::Number(NumberDataType::Int32),
        TypeName::Int64 => DataType::Number(NumberDataType::Int64),
        TypeName::Float32 => DataType::Number(NumberDataType::Float32),
        TypeName::Float64 => DataType::Number(NumberDataType::Float64),
        TypeName::Decimal { precision, scale } => {
            DataType::Decimal(decimal_data_type(*precision, *scale)?)
        }
        TypeName::Date => DataType::Date,
        TypeName::Timestamp => DataType::Timestamp,
        TypeName::TimestampTz => DataType::TimestampTz,
        TypeName::String => DataType::String,
        TypeName::Binary => DataType::Binary,
        TypeName::Bitmap => DataType::Bitmap,
        TypeName::Variant => DataType::Variant,
        TypeName::Geometry => DataType::Geometry,
        TypeName::Geography => DataType::Geography,
        TypeName::Interval => DataType::Interval,
        TypeName::Array(item_type) => {
            DataType::Array(Box::new(resolve_type_name(item_type, not_null)?))
        }
        TypeName::Map { key_type, val_type } => {
            let key_type = resolve_type_name(key_type, true)?;
            let val_type = resolve_type_name(val_type, not_null)?;
            DataType::Map(Box::new(DataType::Tuple(vec![key_type, val_type])))
        }
        TypeName::Tuple { fields_type, .. } => DataType::Tuple(
            fields_type
                .iter()
                .map(|field_type| resolve_type_name(field_type, not_null))
                .collect::<Result<Vec<_>>>()?,
        ),
        TypeName::Nullable(inner) => resolve_type_name(inner, not_null)?.wrap_nullable(),
        TypeName::NotNull(inner) => resolve_type_name(inner, true)?.remove_nullable(),
        TypeName::Vector(dimension) => DataType::Vector(*dimension),
        TypeName::StageLocation => {
            return Err(SqlsmithError::TypeResolution(
                "StageLocation is not supported by sqlsmith DataType".to_string(),
            ));
        }
    };

    if !matches!(type_name, TypeName::Nullable(_) | TypeName::NotNull(_)) && !not_null {
        return Ok(data_type.wrap_nullable());
    }
    Ok(data_type)
}

pub(crate) fn convert_to_type_name(data_type: &DataType) -> TypeName {
    match data_type {
        DataType::Boolean => TypeName::Boolean,
        DataType::Number(NumberDataType::UInt8) => TypeName::UInt8,
        DataType::Number(NumberDataType::UInt16) => TypeName::UInt16,
        DataType::Number(NumberDataType::UInt32) => TypeName::UInt32,
        DataType::Number(NumberDataType::UInt64) => TypeName::UInt64,
        DataType::Number(NumberDataType::Int8) => TypeName::Int8,
        DataType::Number(NumberDataType::Int16) => TypeName::Int16,
        DataType::Number(NumberDataType::Int32) => TypeName::Int32,
        DataType::Number(NumberDataType::Int64) => TypeName::Int64,
        DataType::Number(NumberDataType::Float32) => TypeName::Float32,
        DataType::Number(NumberDataType::Float64) => TypeName::Float64,
        DataType::Decimal(decimal) => {
            let size = decimal.decimal_size();
            TypeName::Decimal {
                precision: size.precision,
                scale: size.scale,
            }
        }
        DataType::Date => TypeName::Date,
        DataType::Timestamp => TypeName::Timestamp,
        DataType::TimestampTz => TypeName::TimestampTz,
        DataType::String => TypeName::String,
        DataType::Bitmap => TypeName::Bitmap,
        DataType::Variant => TypeName::Variant,
        DataType::Binary => TypeName::Binary,
        DataType::Geometry => TypeName::Geometry,
        DataType::Geography => TypeName::Geography,
        DataType::Interval => TypeName::Interval,
        DataType::Vector(dimension) => TypeName::Vector(*dimension),
        DataType::Nullable(inner) => TypeName::Nullable(Box::new(convert_to_type_name(inner))),
        DataType::Array(inner) => TypeName::Array(Box::new(convert_to_type_name(inner))),
        DataType::Map(inner) => match inner.as_ref() {
            DataType::Tuple(fields) if fields.len() == 2 => TypeName::Map {
                key_type: Box::new(convert_to_type_name(&fields[0])),
                val_type: Box::new(convert_to_type_name(&fields[1])),
            },
            _ => TypeName::String,
        },
        DataType::Tuple(fields) => TypeName::Tuple {
            fields_name: None,
            fields_type: fields.iter().map(convert_to_type_name).collect(),
        },
        _ => TypeName::String,
    }
}
