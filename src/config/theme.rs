use serde::{Deserialize, Serialize};

/// ```yaml`
/// theme:
///   tab:
///     active:
///       color:
///         fg: name | hex | integer
///         bg: name | hex | integer
///       modifier: reversed | bold
///     mouse_over:
///       color:
///         fg: name | hex | integer
///         bg: name | hex | integer
///       modifier: reversed | bold
///   component;
///     border:
///       active:
///         color:
///           fg: name | hex | integer
///           bg: name | hex | integer
///         modifier: reversed | bold
///       mouse_over:
///         color:
///           fg: name | hex | integer
///           bg: name | hex | integer
///         modifier: reversed | bold
/// ```
#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct Theme {}
