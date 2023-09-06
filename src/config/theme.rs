use serde::{Deserialize, Serialize};

use crate::{window::ViewTheme, yaml::ColorizedYamlTheme};

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
pub struct Theme {
    #[serde(flatten)]
    pub window: ViewTheme,

    pub colorized_yaml: ColorizedYamlTheme,
}
