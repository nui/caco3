/// Measure given expression usage time.
///
/// If expression is early return, Time will not be measured.
/// It occur when we use `?` in expression.
/// Some kind of that expression can be measured by moving `?` out of expression.
///
/// For example
/// ```ignore
/// // This is async method that return Result
/// async fn get() -> Result<(), ()> { ... }
///
/// async fn main() -> Result<(), ()> {
///     const TAG: &'static str = "Get something";
///     // don't do this, if error occur, measurement log is not emit.
///     measure_time!(TAG, get().await?);
///     // change to this instead.
///     measure_time!(TAG, get().await)?;
/// }
/// ```
///
/// NOTE:
/// * Use `;` as a unit separator cause rustfmt at call site not working properly.
/// * `$tag` can be anything that implement `std::fmt::Display`.
#[macro_export]
macro_rules! measure_time {
    // Custom unit implementation
    (@unit [$unit:literal, $as_unit:ident]; $tag:expr, $expr:expr) => {
        {
            let start = ::std::time::Instant::now();
            let value = $expr;
            $crate::re::tracing::debug!(
                ::core::concat!("{} in {} ", $unit),
                $tag,
                start.elapsed().$as_unit(),
            );
            value
        }
    };
    // Auto unit implementation
    (@auto $tag:expr, $expr:expr) => {
        {
            let start = ::std::time::Instant::now();
            let value = $expr;
            $crate::re::tracing::debug!(
                "{} in {}",
                $tag,
                $crate::_macro_support::AutoUnitDuration::from(start),
            );
            value
        }
    };
    // We usually use this variant
    ($tag:expr, $expr:expr) => { $crate::measure_time!(@auto $tag, $expr) };
    // Use following variants when custom unit is desire
    (MILLI, $tag:expr, $expr:expr) => { $crate::measure_time!(@unit ["ms", as_millis]; $tag, $expr) };
    (MICRO, $tag:expr, $expr:expr) => { $crate::measure_time!(@unit ["µs", as_micros]; $tag, $expr) };
    (NANO,  $tag:expr, $expr:expr) => { $crate::measure_time!(@unit ["ns", as_nanos];  $tag, $expr) };
    (SEC,   $tag:expr, $expr:expr) => { $crate::measure_time!(@unit ["s",  as_secs];   $tag, $expr) };
}


/// Generate database access layer method on given struct.
///
/// This helper macro avoid boilerplate when implement database access layer.
/// For complex sql operation, one should implement it manually.
///
/// ```ignore
/// // Example usage
/// #[derive(sqlx::FromRow)]
/// struct Account {
///     id: i64,
///     name: String,
///     surname: String,
///     active: bool,
/// }
///
/// // Prepared statement arguments
/// struct FindAccount {
///     id: i64,
///     active: bool,
/// }
///
/// impl FindAccount {
///     const FETCH_SQL: &'static str = "select * from accounts where id = ?, and active = ?";
///
///    // Use case 1, Get one record from query result.
///    // -- source --
///    postgres_query! {
///        fetch_one(FindAccount::FETCH_SQL) -> Account,
///        pub async fn get {
///            id,
///        }
///    }
///    // -- expanded --
///    pub async fn get<'c, E>(&self, executor: E) -> sqlx::Result<Account>
///    where
///        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
///    {
///        sqlx::query_as(FindAccount::FETCH_SQL)
///            .bind(&self.id)
///            .fetch_one(executor)
///            .await
///    }
///
///
///     // Use case 2, Find one record from query result.
///     // -- source --
///     postgres_query! {
///         fetch_optional(FindAccount::FETCH_SQL) -> Account,
///         pub async fn find {
///             id,
///             active,
///         }
///     }
///     // -- expanded --
///     pub async fn find<'c, E>(
///         &self,
///         executor: E,
///     ) -> sqlx::Result<Option<Account>>
///     where
///         E: sqlx::Executor<'c, Database = sqlx::Postgres>,
///     {
///         sqlx::query_as(FindAccount::FETCH_SQL)
///             .bind(&self.id)
///             .bind(&self.active)
///             .fetch_optional(executor)
///             .await
///     }
///
///
///     // Use case 3, Get all records from query result.
///     // -- source --
///     postgres_query! {
///         fetch_all(FindAccount::FETCH_SQL) -> Account,
///         pub async fn list {
///             id,
///             active,
///         }
///     }
///     // -- expanded --
///     pub async fn list<'c, E>(
///         &self,
///         executor: E,
///     ) -> sqlx::Result<Vec<Account>>
///     where
///         E: sqlx::Executor<'c, Database = sqlx::Postgres>,
///     {
///         sqlx::query_as(FindAccount::FETCH_SQL)
///             .bind(&self.id)
///             .bind(&self.active)
///             .fetch_all(executor)
///             .await
///     }
/// }
/// ```
#[macro_export]
macro_rules! postgres_query {
    // Hide distracting implementation details from the generated rustdoc.
    ($($body:tt)+) => {
        $crate::postgres_query_internal! {$($body)+}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! postgres_query_internal {
    // internal rules
    (
        @query_impl
        ($query_fn:ident, $execute_fn:ident -> $from_row:ty),
        $sql:expr,
        $(#[$fn_meta:meta])*
        $fn_vis:vis async fn $fn_name:ident ($($field:tt),* $(,)?)
    ) => {
        $(#[$fn_meta])*
        $fn_vis async fn $fn_name<'c, E>(&self, executor: E) -> ::sqlx::Result<$from_row>
        where
            E: ::sqlx::Executor<'c, Database = ::sqlx::Postgres>,
        {
            use ::std::sync::OnceLock;
            use $crate::sql::SqlTrimBoxed;

            // we choose this name to avoid shadowing outer SQL (if exist)
            static __BOXED_QUERY__: OnceLock<Box<str>> = OnceLock::new();

            ::sqlx::$query_fn(
                    &**__BOXED_QUERY__.get_or_init(|| {
                        $sql.sql_trim_boxed()
                    })
                )
                $(.bind(&self.$field))*
                .$execute_fn(executor)
                .await
        }
    };
    // support named struct
    (
        @query
        ($query_fn:ident, $execute_fn:ident -> $from_row:ty),
        $sql:expr,
        $(#[$fn_meta:meta])*
        $fn_vis:vis async fn $fn_name:ident {$($field:ident),* $(,)?}
    ) => {
        $crate::postgres_query_internal! {
            @query_impl
            ($query_fn, $execute_fn -> $from_row),
            $sql,
            $(#[$fn_meta])*
            $fn_vis async fn $fn_name ($($field),*)
        }
    };
    // support tuple struct
    (
        @query
        ($query_fn:ident, $execute_fn:ident -> $from_row:ty),
        $sql:expr,
        $(#[$fn_meta:meta])*
        $fn_vis:vis async fn $fn_name:ident ($($field:tt),* $(,)?)
    ) => {
        $crate::postgres_query_internal! {
            @query_impl
            ($query_fn, $execute_fn -> $from_row),
            $sql,
            $(#[$fn_meta])*
            $fn_vis async fn $fn_name ($($field),*)
        }
    };
    // get one row
    (
        fetch_one($sql:expr) -> $from_row:ty,
        $($fn_spec:tt)*
    ) => {
        $crate::postgres_query_internal! {
            @query
            (query_as, fetch_one -> $from_row),
            $sql,
            $($fn_spec)*
        }
    };
    // get one row with single column
    (
        fetch_one_scalar($sql:expr) -> $from_row:ty,
        $($fn_spec:tt)*
    ) => {
        $crate::postgres_query_internal! {
            @query
            (query_scalar, fetch_one -> $from_row),
            $sql,
            $($fn_spec)*
        }
    };
    // find one row
    (
        fetch_optional($sql:expr) -> $from_row:ty,
        $($fn_spec:tt)*
    ) => {
        $crate::postgres_query_internal! {
            @query
            (query_as, fetch_optional -> ::std::option::Option<$from_row>),
            $sql,
            $($fn_spec)*
        }
    };
    // find one row with single column
    (
        fetch_optional_scalar($sql:expr) -> $from_row:ty,
        $($fn_spec:tt)*
    ) => {
        $crate::postgres_query_internal! {
            @query
            (query_scalar, fetch_optional -> ::std::option::Option<$from_row>),
            $sql,
            $($fn_spec)*
        }
    };
    // fetch all
    (
        fetch_all($sql:expr) -> $from_row:ty,
        $($fn_spec:tt)*
    ) => {
        $crate::postgres_query_internal! {
            @query
            (query_as, fetch_all -> ::std::vec::Vec<$from_row>),
            $sql,
            $($fn_spec)*
        }
    };
    // execute
    (
        execute($sql:expr),
        $($fn_spec:tt)*
    ) => {
        $crate::postgres_query_internal! {
            @query
            (query, execute -> ::sqlx::postgres::PgQueryResult),
            $sql,
            $($fn_spec)*
        }
    };
}


#[macro_export]
macro_rules! sqlite_query {
    // Hide distracting implementation details from the generated rustdoc.
    ($($body:tt)+) => {
        $crate::sqlite_query_internal! {$($body)+}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! sqlite_query_internal {
    // internal rules
    (
        @query_impl
        ($query_fn:ident, $execute_fn:ident -> $entity:ty),
        $sql:expr,
        $(#[$fn_meta:meta])*
        $fn_vis:vis async fn $fn_name:ident ($($field:tt),* $(,)?)
    ) => {
        $(#[$fn_meta])*
        $fn_vis async fn $fn_name<'c, E>(&self, executor: E) -> ::sqlx::Result<$entity>
        where
            E: ::sqlx::Executor<'c, Database = ::sqlx::Sqlite>,
        {
            use ::std::sync::OnceLock;
            use $crate::sql::SqlTrimBoxed;

            // we choose this name to avoid shadowing outer SQL (if exist)
            static __BOXED_QUERY__: OnceLock<Box<str>> = OnceLock::new();
            ::sqlx::$query_fn(&**__BOXED_QUERY__.get_or_init(|| {
                        $sql.sql_trim_boxed()
                    })
                )
                $(.bind(&self.$field))*
                .$execute_fn(executor)
                .await
        }
    };
    // support named struct
    (
        @query
        ($query_fn:ident, $execute_fn:ident -> $entity:ty),
        $sql:expr,
        $(#[$fn_meta:meta])*
        $fn_vis:vis async fn $fn_name:ident {$($field:ident),* $(,)?}
    ) => {
        $crate::sqlite_query_internal! {
            @query_impl
            ($query_fn, $execute_fn -> $entity),
            $sql,
            $(#[$fn_meta])*
            $fn_vis async fn $fn_name ($($field),*)
        }
    };
    // support tuple struct
    (
        @query
        ($query_fn:ident, $execute_fn:ident -> $entity:ty),
        $sql:expr,
        $(#[$fn_meta:meta])*
        $fn_vis:vis async fn $fn_name:ident ($($field:tt),* $(,)?)
    ) => {
        $crate::sqlite_query_internal! {
            @query_impl
            ($query_fn, $execute_fn -> $entity),
            $sql,
            $(#[$fn_meta])*
            $fn_vis async fn $fn_name ($($field),*)
        }
    };
    // get one entity
    (
        get($sql:expr) -> $entity:ty,
        $($fn_spec:tt)*
    ) => {
        $crate::sqlite_query_internal! {
            @query
            (query_as, fetch_one -> $entity),
            $sql,
            $($fn_spec)*
        }
    };
    // get one entity (scalar)
    (
        get_scalar($sql:expr) -> $entity:ty,
        $($fn_spec:tt)*
    ) => {
        $crate::sqlite_query_internal! {
            @query
            (query_scalar, fetch_one -> $entity),
            $sql,
            $($fn_spec)*
        }
    };
    // find one entity
    (
        find($sql:expr) -> $entity:ty,
        $($fn_spec:tt)*
    ) => {
        $crate::sqlite_query_internal! {
            @query
            (query_as, fetch_optional -> ::std::option::Option<$entity>),
            $sql,
            $($fn_spec)*
        }
    };
    // find one entity (scalar)
    (
        find_scalar($sql:expr) -> $entity:ty,
        $($fn_spec:tt)*
    ) => {
        $crate::sqlite_query_internal! {
            @query
            (query_scalar, fetch_optional -> ::std::option::Option<$entity>),
            $sql,
            $($fn_spec)*
        }
    };
    // fetch all
    (
        list($sql:expr) -> $entity:ty,
        $($fn_spec:tt)*
    ) => {
        $crate::sqlite_query_internal! {
            @query
            (query_as, fetch_all -> ::std::vec::Vec<$entity>),
            $sql,
            $($fn_spec)*
        }
    };
    // execute
    (
        execute($sql:expr),
        $($fn_spec:tt)*
    ) => {
        $crate::sqlite_query_internal! {
            @query
            (query, execute -> ::sqlx::sqlite::SqliteQueryResult),
            $sql,
            $($fn_spec)*
        }
    };
}


/// Generate `builder()` method which return builder with default values.
#[macro_export]
macro_rules! with_builder {
    ($builder:ty => $ty:ty) => {
        impl $ty {
            pub fn builder() -> $builder {
                <$builder as ::core::default::Default>::default()
            }
        }
    };
}

/// Generating function used for reading jemalloc stats.
///
/// Unfortunately we couldn't re-export jemalloc struct so we hard coded its path here.
#[macro_export]
macro_rules! generate_read_jemalloc_raw_data {
    ($vis:vis fn $name:ident) => {
        $vis fn $name() -> ::core::option::Option<$crate::jemalloc::info::JemallocRawData> {
            use ::std::prelude::*;
            use tikv_jemalloc_ctl::{arenas, background_thread, epoch, max_background_threads, stats};

            use $crate::jemalloc::info::{JemallocRawData, BackgroundThread};

            fn read_background_thread() -> Option<BackgroundThread> {
                Some(BackgroundThread {
                    max: max_background_threads::read().ok()?,
                    enabled: background_thread::read().ok()?,
                })
            }
            // Many statistics are cached and only updated
            // when the epoch is advanced:
            epoch::advance().ok()?;
            let value = JemallocRawData {
                // config
                background_thread: read_background_thread(),
                number_of_arenas: arenas::narenas::read().ok()?,
                // stats
                active_bytes: stats::active::read().ok()?,
                allocated_bytes: stats::allocated::read().ok()?,
                mapped_bytes: stats::mapped::read().ok()?,
                metadata_bytes: stats::metadata::read().ok()?,
                resident_bytes: stats::resident::read().ok()?,
                retained_bytes: stats::retained::read().ok()?,
            };
            Some(value)
        }
    };
}
