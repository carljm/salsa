mod accumulator;
mod alloc;
mod array;
mod cancelled;
mod cycle;
mod database;
mod durability;
mod event;
mod function;
mod handle;
mod hash;
mod id;
mod ingredient;
mod ingredient_list;
mod input;
mod interned;
mod key;
mod nonce;
mod revision;
mod runtime;
mod salsa_struct;
mod storage;
mod tracked_struct;
mod update;
mod views;

pub use self::accumulator::Accumulator;
pub use self::cancelled::Cancelled;
pub use self::cycle::Cycle;
pub use self::database::Database;
pub use self::durability::Durability;
pub use self::event::Event;
pub use self::event::EventKind;
pub use self::handle::Handle;
pub use self::id::Id;
pub use self::input::setter::Setter;
pub use self::key::DatabaseKeyIndex;
pub use self::revision::Revision;
pub use self::runtime::Runtime;
pub use self::storage::Storage;
pub use self::update::Update;
pub use crate::database::with_attached_database;
pub use salsa_macros::accumulator;
pub use salsa_macros::db;
pub use salsa_macros::input;
pub use salsa_macros::interned;
pub use salsa_macros::tracked;
pub use salsa_macros::Update;

pub fn default_database() -> impl Database {
    use crate as salsa;

    #[crate::db]
    #[derive(Default)]
    struct DefaultDatabase {
        storage: Storage<Self>,
    }

    #[crate::db]
    impl Database for DefaultDatabase {}

    DefaultDatabase::default()
}

pub mod prelude {
    pub use crate::Accumulator;
    pub use crate::Setter;
}

/// Internal names used by salsa macros.
///
/// # WARNING
///
/// The contents of this module are NOT subject to semver.
pub mod plumbing {
    pub use crate::accumulator::Accumulator;
    pub use crate::array::Array;
    pub use crate::cycle::Cycle;
    pub use crate::cycle::CycleRecoveryStrategy;
    pub use crate::database::attach_database;
    pub use crate::database::current_revision;
    pub use crate::database::with_attached_database;
    pub use crate::database::Database;
    pub use crate::function::should_backdate_value;
    pub use crate::id::AsId;
    pub use crate::id::FromId;
    pub use crate::id::Id;
    pub use crate::id::LookupId;
    pub use crate::ingredient::Ingredient;
    pub use crate::ingredient::Jar;
    pub use crate::key::DatabaseKeyIndex;
    pub use crate::revision::Revision;
    pub use crate::runtime::stamp;
    pub use crate::runtime::Runtime;
    pub use crate::runtime::Stamp;
    pub use crate::runtime::StampedValue;
    pub use crate::salsa_struct::SalsaStructInDb;
    pub use crate::storage::views;
    pub use crate::storage::HasStorage;
    pub use crate::storage::IngredientCache;
    pub use crate::storage::IngredientIndex;
    pub use crate::storage::Storage;
    pub use crate::tracked_struct::TrackedStructInDb;
    pub use crate::update::always_update;
    pub use crate::update::helper::Dispatch as UpdateDispatch;
    pub use crate::update::helper::Fallback as UpdateFallback;
    pub use crate::update::Update;

    pub use salsa_macro_rules::macro_if;
    pub use salsa_macro_rules::maybe_backdate;
    pub use salsa_macro_rules::maybe_clone;
    pub use salsa_macro_rules::maybe_cloned_ty;
    pub use salsa_macro_rules::setup_accumulator_impl;
    pub use salsa_macro_rules::setup_input_struct;
    pub use salsa_macro_rules::setup_interned_struct;
    pub use salsa_macro_rules::setup_method_body;
    pub use salsa_macro_rules::setup_tracked_fn;
    pub use salsa_macro_rules::setup_tracked_struct;
    pub use salsa_macro_rules::unexpected_cycle_recovery;

    pub mod accumulator {
        pub use crate::accumulator::IngredientImpl;
        pub use crate::accumulator::JarImpl;
    }

    pub mod input {
        pub use crate::input::input_field::FieldIngredientImpl;
        pub use crate::input::setter::SetterImpl;
        pub use crate::input::Configuration;
        pub use crate::input::IngredientImpl;
        pub use crate::input::JarImpl;
    }

    pub mod interned {
        pub use crate::interned::Configuration;
        pub use crate::interned::IngredientImpl;
        pub use crate::interned::JarImpl;
        pub use crate::interned::Value;
    }

    pub mod function {
        pub use crate::function::Configuration;
        pub use crate::function::IngredientImpl;
    }

    pub mod tracked_struct {
        pub use crate::tracked_struct::tracked_field::FieldIngredientImpl;
        pub use crate::tracked_struct::Configuration;
        pub use crate::tracked_struct::IngredientImpl;
        pub use crate::tracked_struct::JarImpl;
        pub use crate::tracked_struct::Value;
    }
}
