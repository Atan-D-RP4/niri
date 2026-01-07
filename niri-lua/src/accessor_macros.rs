macro_rules! accessor_bool {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| Ok(LuaValue::Boolean(config.$($field).+)),
            |_lua, config, value| {
                config.$($field).+ = bool::from_lua(value, _lua)?;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_option_bool {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| match config.$($field).+ {
                Some(v) => Ok(LuaValue::Boolean(v)),
                None => Ok(LuaValue::Nil),
            },
            |_lua, config, value| {
                config.$($field).+ = Option::<bool>::from_lua(value, _lua)?;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_int {
    ($registry:expr, $path:literal, $($field:ident).+, $ty:ty) => {
        $registry.update_accessor(
            $path,
            |_lua, config| Ok(LuaValue::Integer(config.$($field).+ as i64)),
            |_lua, config, value| {
                config.$($field).+ = <$ty>::from_lua(value, _lua)?;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_float {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| Ok(LuaValue::Number(config.$($field).+)),
            |_lua, config, value| {
                config.$($field).+ = f64::from_lua(value, _lua)?;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_string {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| Ok(LuaValue::String(_lua.create_string(&config.$($field).+)?)),
            |_lua, config, value| {
                config.$($field).+ = String::from_lua(value, _lua)?;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_option_int {
    ($registry:expr, $path:literal, $($field:ident).+, $ty:ty) => {
        $registry.update_accessor(
            $path,
            |_lua, config| match config.$($field).+ {
                Some(v) => Ok(LuaValue::Integer(v as i64)),
                None => Ok(LuaValue::Nil),
            },
            |_lua, config, value| {
                config.$($field).+ = Option::<$ty>::from_lua(value, _lua)?;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_option_string {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| match &config.$($field).+ {
                Some(v) => Ok(LuaValue::String(_lua.create_string(v)?)),
                None => Ok(LuaValue::Nil),
            },
            |_lua, config, value| {
                config.$($field).+ = Option::<String>::from_lua(value, _lua)?;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_enum {
    ($registry:expr, $path:literal, $($field:ident).+, $enum_ty:ty, $($variant:ident => $str:literal),+ $(,)?) => {
        $registry.update_accessor(
            $path,
            |_lua, config| match config.$($field).+ {
                $(<$enum_ty>::$variant => Ok(LuaValue::String(_lua.create_string($str)?)),)+
            },
            |_lua, config, value| {
                let v = String::from_lua(value, _lua)?;
                config.$($field).+ = match v.as_str() {
                    $($str => <$enum_ty>::$variant,)+
                    other => return Err(LuaError::external(format!(
                        "invalid {} value: {}", $path, other
                    ))),
                };
                Ok(())
            },
        );
    };
}

macro_rules! accessor_option_enum {
    ($registry:expr, $path:literal, $($field:ident).+, $enum_ty:ty, $($variant:ident => $str:literal),+ $(,)?) => {
        $registry.update_accessor(
            $path,
            |_lua, config| match config.$($field).+ {
                $(Some(<$enum_ty>::$variant) => Ok(LuaValue::String(_lua.create_string($str)?)),)+
                None => Ok(LuaValue::Nil),
            },
            |_lua, config, value| {
                let v = Option::<String>::from_lua(value, _lua)?;
                config.$($field).+ = match v.as_deref() {
                    $( Some($str) => Some(<$enum_ty>::$variant), )+
                    Some(other) => return Err(LuaError::external(format!(
                        "invalid {} value: {}", $path, other
                    ))),
                    None => None,
                };
                Ok(())
            },
        );
    };
}

macro_rules! accessor_option_enum_normalize {
    ($registry:expr, $path:literal, $($field:ident).+, $enum_ty:ty, $($variant:ident => $str:literal),+ $(,)?) => {
        $registry.update_accessor(
            $path,
            |_lua, config| match config.$($field).+ {
                $(Some(<$enum_ty>::$variant) => Ok(LuaValue::String(_lua.create_string($str)?)),)+
                None => Ok(LuaValue::Nil),
            },
            |_lua, config, value| {
                let v = Option::<String>::from_lua(value, _lua)?;
                config.$($field).+ = match v.as_deref().map(|s| s.replace('-', "_").to_lowercase()).as_deref() {
                    $( Some($str) => Some(<$enum_ty>::$variant), )+
                    Some(other) => return Err(LuaError::external(format!(
                        "invalid {} value: {}", $path, other
                    ))),
                    None => None,
                };
                Ok(())
            },
        );
    };
}

macro_rules! accessor_float_or_int {
    ($registry:expr, $path:literal, $($field:ident).+, $min:literal, $max:literal) => {
        $registry.update_accessor(
            $path,
            |_lua, config| Ok(LuaValue::Number(config.$($field).+.0)),
            |_lua, config, value| {
                let v = f64::from_lua(value, _lua)?;
                config.$($field).+ = niri_config::utils::FloatOrInt::<$min, $max>(v);
                Ok(())
            },
        );
    };
}

macro_rules! accessor_color {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| {
                let color = &config.$($field).+;
                Ok(LuaValue::String(_lua.create_string(&format!(
                    "#{:02x}{:02x}{:02x}{:02x}",
                    (color.r * 255.) as u8,
                    (color.g * 255.) as u8,
                    (color.b * 255.) as u8,
                    (color.a * 255.) as u8,
                ))?))
            },
            |_lua, config, value| {
                let v = String::from_lua(value, _lua)?;
                let color = $crate::traits::parse_color_string(&v)?;
                config.$($field).+ = color;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_option_color {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| match &config.$($field).+ {
                Some(color) => Ok(LuaValue::String(_lua.create_string(&format!(
                    "#{:02x}{:02x}{:02x}{:02x}",
                    (color.r * 255.) as u8,
                    (color.g * 255.) as u8,
                    (color.b * 255.) as u8,
                    (color.a * 255.) as u8,
                ))?)),
                None => Ok(LuaValue::Nil),
            },
            |_lua, config, value| {
                if value.is_nil() {
                    config.$($field).+ = None;
                } else {
                    let v = String::from_lua(value, _lua)?;
                    let color = $crate::traits::parse_color_string(&v)?;
                    config.$($field).+ = Some(color);
                }
                Ok(())
            },
        );
    };
}

macro_rules! accessor_inverted_bool {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor_with_type(
            $path,
            $crate::property_registry::PropertyType::Bool,
            |_lua, config| Ok(LuaValue::Boolean(!config.$($field).+)),
            |_lua, config, value| {
                config.$($field).+ = !bool::from_lua(value, _lua)?;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_option_path {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| match &config.$($field).+ {
                Some(path) => Ok(LuaValue::String(
                    _lua.create_string(&*path.to_string_lossy())?,
                )),
                None => Ok(LuaValue::Nil),
            },
            |_lua, config, value| {
                let v = Option::<String>::from_lua(value, _lua)?;
                config.$($field).+ = v.map(std::path::PathBuf::from);
                Ok(())
            },
        );
    };
}

macro_rules! accessor_wrapped_option_string {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| match &config.$($field).+.0 {
                Some(v) => Ok(LuaValue::String(_lua.create_string(v)?)),
                None => Ok(LuaValue::Nil),
            },
            |_lua, config, value| {
                config.$($field).+.0 = Option::<String>::from_lua(value, _lua)?;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_option_gradient {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| match &config.$($field).+ {
                Some(gradient) => {
                    let table = $crate::traits::gradient_to_table(_lua, gradient)?;
                    Ok(LuaValue::Table(table))
                }
                None => Ok(LuaValue::Nil),
            },
            |_lua, config, value| {
                if value.is_nil() {
                    config.$($field).+ = None;
                } else {
                    let table = mlua::Table::from_lua(value, _lua)?;
                    let gradient = $crate::traits::table_to_gradient(table)?;
                    config.$($field).+ = Some(gradient);
                }
                Ok(())
            },
        );
    };
}

macro_rules! accessor_anim_kind {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| {
                let table = $crate::traits::anim_kind_to_table(_lua, &config.$($field).+.0.kind)?;
                Ok(LuaValue::Table(table))
            },
            |_lua, config, value| {
                let table = mlua::Table::from_lua(value, _lua)?;
                let kind = $crate::traits::table_to_anim_kind(table)?;
                config.$($field).+.0.kind = kind;
                Ok(())
            },
        );
    };
}

macro_rules! accessor_anim_kind_named {
    ($registry:expr, $path:literal, $($field:ident).+) => {
        $registry.update_accessor(
            $path,
            |_lua, config| {
                let table = $crate::traits::anim_kind_to_table(_lua, &config.$($field).+.anim.kind)?;
                Ok(LuaValue::Table(table))
            },
            |_lua, config, value| {
                let table = mlua::Table::from_lua(value, _lua)?;
                let kind = $crate::traits::table_to_anim_kind(table)?;
                config.$($field).+.anim.kind = kind;
                Ok(())
            },
        );
    };
}

pub(crate) use {
    accessor_anim_kind, accessor_anim_kind_named, accessor_bool, accessor_color, accessor_enum,
    accessor_float, accessor_float_or_int, accessor_int, accessor_inverted_bool,
    accessor_option_bool, accessor_option_color, accessor_option_enum,
    accessor_option_enum_normalize, accessor_option_gradient, accessor_option_int,
    accessor_option_path, accessor_option_string, accessor_string, accessor_wrapped_option_string,
};
