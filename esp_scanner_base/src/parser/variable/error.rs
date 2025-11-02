use crate::types::common::DataType;
use crate::types::runtime_operation::RuntimeOperationType;

pub enum VariableResolutionError {
    RunOperationFailed {
        variable: String,
        operation: RuntimeOperationType,
        cause: String,
    },
    InvalidJsonValue {
        variable: String,
        json_content: String,
    },
    TypeCoercionFailed {
        from_type: String,
        to_type: DataType,
    },
}
