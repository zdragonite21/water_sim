pub trait DebugStats {
    fn append_debug_text_rows(&self, rows: &mut Vec<String>);
}

macro_rules! debug_stats {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $($body:tt)*
        }
    ) => {
        debug_stats! {
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
        [$(($field:ident, $ty:ty, $default:expr))*]
        [$(($stat_field:ident))*]
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

        impl crate::stats::DebugStats for $name {
            fn append_debug_text_rows(&self, rows: &mut Vec<String>) {
                let _ = &rows;
                $(
                    rows.push(format!("{}: {}", stringify!($stat_field), self.$stat_field));
                )*
            }
        }
    };

    (
        @parse
        [$($meta:meta),*]
        $name:ident
        [$(($field:ident, $ty:ty, $default:expr))*]
        [$(($stat_field:ident))*]
        stat $next_field:ident: $next_ty:ty = $next_default:expr;
        $($rest:tt)*
    ) => {
        debug_stats! {
            @parse
            [$($meta),*]
            $name
            [$(($field, $ty, $default))* ($next_field, $next_ty, $next_default)]
            [$(($stat_field))* ($next_field)]
            $($rest)*
        }
    };

    (
        @parse
        [$($meta:meta),*]
        $name:ident
        [$(($field:ident, $ty:ty, $default:expr))*]
        [$(($stat_field:ident))*]
        $next_field:ident: $next_ty:ty = $next_default:expr;
        $($rest:tt)*
    ) => {
        debug_stats! {
            @parse
            [$($meta),*]
            $name
            [$(($field, $ty, $default))* ($next_field, $next_ty, $next_default)]
            [$(($stat_field))*]
            $($rest)*
        }
    };
}

pub(crate) use debug_stats;
