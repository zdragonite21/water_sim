pub trait Inspect {
    fn inspect(&mut self, ui: &mut egui::Ui);
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
            fn inspect(&mut self, ui: &mut egui::Ui) {
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
    ($ui:ident, $self:ident,) => {
        let _ = $ui;
    };
    ($ui:ident, $self:ident,) => {};
    (
        $ui:ident,
        $self:ident,
        (checkbox, bool, $field:ident, $default:expr, [$($extra:tt)*])
        (checkbox, bool, $next_field:ident, $next_default:expr, [$($next_extra:tt)*])
        $($rest:tt)*
    ) => {
        $ui.horizontal(|$ui| {
            $crate::inspect::inspect_config_draw_field!(
                $ui,
                $self,
                checkbox,
                bool,
                $field,
                $default $($extra)*
            );
            $crate::inspect::inspect_config_draw_field!(
                $ui,
                $self,
                checkbox,
                bool,
                $next_field,
                $next_default $($next_extra)*
            );
        });
        $crate::inspect::inspect_config_draw_fields!(
            $ui,
            $self,
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
        $ui.add(egui::Slider::new(&mut $self.$field, $min..=$max).text(stringify!($field)));
    };
    ($ui:ident, $self:ident, slider, f32, $field:ident, $default:expr) => {
        let slider_range = (2.0 * ($default as f32).abs()).max(1.0);
        $ui.add(
            egui::Slider::new(&mut $self.$field, -slider_range..=slider_range)
                .text(stringify!($field)),
        );
    };
    ($ui:ident, $self:ident, slider, u32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        $ui.add(egui::Slider::new(&mut $self.$field, $min..=$max).text(stringify!($field)));
    };
    ($ui:ident, $self:ident, slider, u32, $field:ident, $default:expr) => {
        let slider_range = (2.0 * ($default as f32).abs()).max(1.0) as u32;
        $ui.add(egui::Slider::new(&mut $self.$field, 0..=slider_range).text(stringify!($field)));
    };
    ($ui:ident, $self:ident, slider, i32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        $ui.add(egui::Slider::new(&mut $self.$field, $min..=$max).text(stringify!($field)));
    };
    ($ui:ident, $self:ident, slider, i32, $field:ident, $default:expr) => {
        let slider_range = (2.0 * ($default as f32).abs()).max(1.0) as i32;
        $ui.add(
            egui::Slider::new(&mut $self.$field, -slider_range..=slider_range)
                .text(stringify!($field)),
        );
    };
    ($ui:ident, $self:ident, drag, f32, $field:ident, $default:expr) => {
        $crate::inspect::drag_value($ui, stringify!($field), &mut $self.$field, 1.0, None);
    };
    ($ui:ident, $self:ident, drag, f32, $field:ident, $default:expr, $speed:expr) => {
        $crate::inspect::drag_value(
            $ui,
            stringify!($field),
            &mut $self.$field,
            $speed as f64,
            None,
        );
    };
    ($ui:ident, $self:ident, drag, f32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        let drag_speed = (($max as f32) - ($min as f32)).abs() / 100.0;
        $crate::inspect::drag_value(
            $ui,
            stringify!($field),
            &mut $self.$field,
            drag_speed as f64,
            Some($min..=$max),
        );
    };
    ($ui:ident, $self:ident, drag, u32, $field:ident, $default:expr) => {
        $crate::inspect::drag_value($ui, stringify!($field), &mut $self.$field, 1.0, None);
    };
    ($ui:ident, $self:ident, drag, u32, $field:ident, $default:expr, $speed:expr) => {
        $crate::inspect::drag_value(
            $ui,
            stringify!($field),
            &mut $self.$field,
            $speed as f64,
            None,
        );
    };
    ($ui:ident, $self:ident, drag, u32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        let drag_speed = (($max as f32) - ($min as f32)).abs() / 100.0;
        $crate::inspect::drag_value(
            $ui,
            stringify!($field),
            &mut $self.$field,
            drag_speed as f64,
            Some($min..=$max),
        );
    };
    ($ui:ident, $self:ident, drag, i32, $field:ident, $default:expr) => {
        $crate::inspect::drag_value($ui, stringify!($field), &mut $self.$field, 1.0, None);
    };
    ($ui:ident, $self:ident, drag, i32, $field:ident, $default:expr, $speed:expr) => {
        $crate::inspect::drag_value(
            $ui,
            stringify!($field),
            &mut $self.$field,
            $speed as f64,
            None,
        );
    };
    ($ui:ident, $self:ident, drag, i32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        let drag_speed = (($max as f32) - ($min as f32)).abs() / 100.0;
        $crate::inspect::drag_value(
            $ui,
            stringify!($field),
            &mut $self.$field,
            drag_speed as f64,
            Some($min..=$max),
        );
    };
    ($ui:ident, $self:ident, vector3, [f32; 3], $field:ident, $default:expr) => {
        $crate::inspect::drag_vector3($ui, stringify!($field), &mut $self.$field, 1.0, None);
    };
    ($ui:ident, $self:ident, vector3, [f32; 3], $field:ident, $default:expr, $speed:expr) => {
        $crate::inspect::drag_vector3(
            $ui,
            stringify!($field),
            &mut $self.$field,
            $speed as f64,
            None,
        );
    };
    ($ui:ident, $self:ident, vector3, [f32; 3], $field:ident, $default:expr, $min:expr, $max:expr) => {
        let drag_speed = (($max as f32) - ($min as f32)).abs() / 100.0;
        $crate::inspect::drag_vector3(
            $ui,
            stringify!($field),
            &mut $self.$field,
            drag_speed as f64,
            Some($min..=$max),
        );
    };
    ($ui:ident, $self:ident, color4, [f32; 4], $field:ident, $default:expr) => {
        $ui.horizontal(|ui| {
            ui.label(stringify!($field));
            ui.color_edit_button_rgba_unmultiplied(&mut $self.$field);
        });
    };
    ($ui:ident, $self:ident, checkbox, bool, $field:ident, $default:expr) => {
        $ui.checkbox(&mut $self.$field, stringify!($field));
    };
}

pub fn drag_value<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    speed: f64,
    range: Option<std::ops::RangeInclusive<T>>,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut drag = egui::DragValue::new(value).speed(speed);
        if let Some(range) = range {
            drag = drag.range(range);
        }
        ui.add(drag);
    });
}

pub fn drag_vector3(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut [f32; 3],
    speed: f64,
    range: Option<std::ops::RangeInclusive<f32>>,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.push_id(label, |ui| {
            for component in value {
                let mut drag = egui::DragValue::new(component).speed(speed);
                if let Some(range) = range.clone() {
                    drag = drag.range(range);
                }
                ui.add(drag);
            }
        });
    });
}

pub(crate) use inspect_config;
pub(crate) use inspect_config_draw_field;
pub(crate) use inspect_config_draw_fields;
