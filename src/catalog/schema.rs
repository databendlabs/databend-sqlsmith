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

use std::sync::Arc;

use crate::catalog::types::DataType;
use crate::error::Result;
use crate::error::SqlsmithError;

#[derive(Clone, Debug)]
pub(crate) struct TableField {
    pub(crate) name: String,
    pub(crate) data_type: DataType,
}

impl TableField {
    pub(crate) fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn data_type(&self) -> &DataType {
        &self.data_type
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TableSchema {
    pub(crate) fields: Vec<TableField>,
}

pub(crate) type TableSchemaRef = Arc<TableSchema>;

impl TableSchema {
    pub(crate) fn new(fields: Vec<TableField>) -> Self {
        Self { fields }
    }

    pub(crate) fn new_ref(fields: Vec<TableField>) -> TableSchemaRef {
        Arc::new(Self::new(fields))
    }

    pub(crate) fn fields(&self) -> &[TableField] {
        &self.fields
    }

    pub(crate) fn num_fields(&self) -> usize {
        self.fields.len()
    }

    pub(crate) fn index_of(&self, name: &str) -> Result<usize> {
        self.fields
            .iter()
            .rposition(|field| field.name == name)
            .ok_or_else(|| SqlsmithError::BadArguments(format!("unknown field: {name}")))
    }
}
