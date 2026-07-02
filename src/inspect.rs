pub trait Inspect {
    fn inspect(&mut self, ui: &imgui::Ui);
}

macro_rules! inspect_config {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $($body:tt)*
        }
    ) => {
        inspect_config! {
            @parse
            [$($meta),*]
            $name
            []
            []
            $($body)*
        }
    };

    (
        @parse
        [$($meta:meta),*]
        $name:ident
        [$(($field:ident $tt:tt $default:expr))*]
        [$(($widget:ident $draw_field:ident $draw_tt:tt $draw_default:expr, [$($draw_extra:tt)*]))*]
    ) => {
        $(#[$meta])*
        pub struct $name {
            $(
                pub $field: $tt,
            )*
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    $(
                        $field: $default,
                    )*
                }
            }
        }

        impl crate::inspect::Inspect for $name {
            fn inspect(&mut self, ui: &imgui::Ui) {
                $crate::inspect::inspect_config_draw_fields!(
                    ui,
                    self,
                    $(($widget, $draw_tt, $draw_field, $draw_default, [$($draw_extra)*]))*
                );
            }
        }
    };

    (
        @parse
        [$($meta:meta),*]
        $name:ident
        [$(($field:ident $tt:tt $default:expr))*]
        [$(($widget:ident $draw_field:ident $draw_tt:tt $draw_default:expr, [$($draw_extra:tt)*]))*]
        $next_widget:ident $next_field:ident: $next_tt:tt = $next_default:expr, $first:expr, $second:expr;
        $($rest:tt)*
    ) => {
        inspect_config! {
            @parse
            [$($meta),*]
            $name
            [$(($field $tt $default))* ($next_field $next_tt $next_default)]
            [
                $(($widget $draw_field $draw_tt $draw_default, [$($draw_extra)*]))*
                ($next_widget $next_field $next_tt $next_default, [, $first, $second])
            ]
            $($rest)*
        }
    };

    (
        @parse
        [$($meta:meta),*]
        $name:ident
        [$(($field:ident $tt:tt $default:expr))*]
        [$(($widget:ident $draw_field:ident $draw_tt:tt $draw_default:expr, [$($draw_extra:tt)*]))*]
        $next_widget:ident $next_field:ident: $next_tt:tt = $next_default:expr, $first:expr;
        $($rest:tt)*
    ) => {
        inspect_config! {
            @parse
            [$($meta),*]
            $name
            [$(($field $tt $default))* ($next_field $next_tt $next_default)]
            [
                $(($widget $draw_field $draw_tt $draw_default, [$($draw_extra)*]))*
                ($next_widget $next_field $next_tt $next_default, [, $first])
            ]
            $($rest)*
        }
    };

    (
        @parse
        [$($meta:meta),*]
        $name:ident
        [$(($field:ident $tt:tt $default:expr))*]
        [$(($widget:ident $draw_field:ident $draw_tt:tt $draw_default:expr, [$($draw_extra:tt)*]))*]
        $next_widget:ident $next_field:ident: $next_tt:tt = $next_default:expr;
        $($rest:tt)*
    ) => {
        inspect_config! {
            @parse
            [$($meta),*]
            $name
            [$(($field $tt $default))* ($next_field $next_tt $next_default)]
            [
                $(($widget $draw_field $draw_tt $draw_default, [$($draw_extra)*]))*
                ($next_widget $next_field $next_tt $next_default, [])
            ]
            $($rest)*
        }
    };

    (
        @parse
        [$($meta:meta),*]
        $name:ident
        [$(($field:ident $tt:tt $default:expr))*]
        [$(($widget:ident $draw_field:ident $draw_tt:tt $draw_default:expr, [$($draw_extra:tt)*]))*]
        $next_field:ident: $next_tt:tt = $next_default:expr;
        $($rest:tt)*
    ) => {
        inspect_config! {
            @parse
            [$($meta),*]
            $name
            [$(($field $tt $default))* ($next_field $next_tt $next_default)]
            [$(($widget $draw_field $draw_tt $draw_default, [$($draw_extra)*]))*]
            $($rest)*
        }
    };
}

macro_rules! inspect_config_draw_fields {
    ($ui:ident, $self:ident,) => {};
    (
        $ui:ident,
        $self:ident,
        (checkbox, bool, $field:ident, $default:expr, [$($extra:tt)*])
        (checkbox, bool, $next_field:ident, $next_default:expr, [$($next_extra:tt)*])
        $($rest:tt)*
    ) => {
        $crate::inspect::inspect_config_draw_field!(
            $ui,
            $self,
            checkbox,
            bool,
            $field,
            $default $($extra)*
        );
        $ui.same_line();
        $crate::inspect::inspect_config_draw_fields!(
            $ui,
            $self,
            (checkbox, bool, $next_field, $next_default, [$($next_extra)*])
            $($rest)*
        );
    };
    (
        $ui:ident,
        $self:ident,
        ($widget:ident, $tt:tt, $field:ident, $default:expr, [$($extra:tt)*])
        $($rest:tt)*
    ) => {
        $crate::inspect::inspect_config_draw_field!(
            $ui,
            $self,
            $widget,
            $tt,
            $field,
            $default $($extra)*
        );
        $crate::inspect::inspect_config_draw_fields!($ui, $self, $($rest)*);
    };
}

macro_rules! inspect_config_draw_field {
    ($ui:ident, $self:ident, slider, f32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        $ui.slider(stringify!($field), $min, $max, &mut $self.$field);
    };
    ($ui:ident, $self:ident, slider, f32, $field:ident, $default:expr) => {
        let slider_range = (2.0 * ($default as f32).abs()).max(1.0);
        $ui.slider(
            stringify!($field),
            -slider_range,
            slider_range,
            &mut $self.$field,
        );
    };
    ($ui:ident, $self:ident, slider, u32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        $ui.slider(stringify!($field), $min, $max, &mut $self.$field);
    };
    ($ui:ident, $self:ident, slider, u32, $field:ident, $default:expr) => {
        let slider_range = (2.0 * ($default as f32).abs()).max(1.0) as u32;
        $ui.slider(stringify!($field), 0, slider_range, &mut $self.$field);
    };
    ($ui:ident, $self:ident, slider, i32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        $ui.slider(stringify!($field), $min, $max, &mut $self.$field);
    };
    ($ui:ident, $self:ident, slider, i32, $field:ident, $default:expr) => {
        let slider_range = (2.0 * ($default as f32).abs()).max(1.0) as i32;
        $ui.slider(
            stringify!($field),
            -slider_range,
            slider_range,
            &mut $self.$field,
        );
    };
    ($ui:ident, $self:ident, drag, f32, $field:ident, $default:expr) => {
        imgui::Drag::<f32, _>::new(stringify!($field)).build($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, drag, f32, $field:ident, $default:expr, $speed:expr) => {
        imgui::Drag::<f32, _>::new(stringify!($field))
            .speed($speed as f32)
            .build($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, drag, f32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        let drag_speed = (($max as f32) - ($min as f32)).abs() / 100.0;
        imgui::Drag::<f32, _>::new(stringify!($field))
            .speed(drag_speed)
            .range($min, $max)
            .build($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, drag, u32, $field:ident, $default:expr) => {
        imgui::Drag::<u32, _>::new(stringify!($field)).build($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, drag, u32, $field:ident, $default:expr, $speed:expr) => {
        imgui::Drag::<u32, _>::new(stringify!($field))
            .speed($speed as f32)
            .build($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, drag, u32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        let drag_speed = (($max as f32) - ($min as f32)).abs() / 100.0;
        imgui::Drag::<u32, _>::new(stringify!($field))
            .speed(drag_speed)
            .range($min, $max)
            .build($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, drag, i32, $field:ident, $default:expr) => {
        imgui::Drag::<i32, _>::new(stringify!($field)).build($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, drag, i32, $field:ident, $default:expr, $speed:expr) => {
        imgui::Drag::<i32, _>::new(stringify!($field))
            .speed($speed as f32)
            .build($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, drag, i32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        let drag_speed = (($max as f32) - ($min as f32)).abs() / 100.0;
        imgui::Drag::<i32, _>::new(stringify!($field))
            .speed(drag_speed)
            .range($min, $max)
            .build($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, vector3, [f32; 3], $field:ident, $default:expr) => {
        imgui::Drag::<f32, _>::new(stringify!($field)).build_array($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, vector3, [f32; 3], $field:ident, $default:expr, $speed:expr) => {
        imgui::Drag::<f32, _>::new(stringify!($field))
            .speed($speed as f32)
            .build_array($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, vector3, [f32; 3], $field:ident, $default:expr, $min:expr, $max:expr) => {
        let drag_speed = (($max as f32) - ($min as f32)).abs() / 100.0;
        imgui::Drag::<f32, _>::new(stringify!($field))
            .speed(drag_speed)
            .range($min, $max)
            .build_array($ui, &mut $self.$field);
    };
    ($ui:ident, $self:ident, color4, [f32; 4], $field:ident, $default:expr) => {
        $ui.color_edit4(stringify!($field), &mut $self.$field);
    };
    ($ui:ident, $self:ident, checkbox, bool, $field:ident, $default:expr) => {
        $ui.checkbox(stringify!($field), &mut $self.$field);
    };
}

pub(crate) use inspect_config;
pub(crate) use inspect_config_draw_field;
pub(crate) use inspect_config_draw_fields;
