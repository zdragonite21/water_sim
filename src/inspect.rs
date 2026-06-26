pub trait Inspect {
    fn inspect(&mut self, ui: &imgui::Ui);
}

macro_rules! inspect_config {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $field:ident: $ty:ident = $default:expr $(, $min:expr, $max:expr)?;
            )*
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            $(
                pub $field: $ty,
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
                $(
                    $crate::inspect::inspect_config_draw_field!(
                        ui,
                        self,
                        $ty,
                        $field,
                        $default $(, $min, $max)?
                    );
                )*
            }
        }
    };
}

macro_rules! inspect_config_draw_field {
    ($ui:ident, $self:ident, f32, $field:ident, $default:expr, $min:expr, $max:expr) => {
        $ui.slider(stringify!($field), $min, $max, &mut $self.$field);
    };
    ($ui:ident, $self:ident, f32, $field:ident, $default:expr) => {
        let slider_range = (2.0 * ($default as f32).abs()).max(1.0);
        $ui.slider(
            stringify!($field),
            -slider_range,
            slider_range,
            &mut $self.$field,
        );
    };
    ($ui:ident, $self:ident, bool, $field:ident, $default:expr) => {
        $ui.checkbox(stringify!($field), &mut $self.$field);
    };
}

pub(crate) use inspect_config;
pub(crate) use inspect_config_draw_field;
