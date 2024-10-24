use std::any::{Any, TypeId};

use orx_concurrent_vec::ConcurrentVec;
use parking_lot::Mutex;
use rustc_hash::FxHashMap;

use crate::cycle::CycleRecoveryStrategy;
use crate::ingredient::{Ingredient, Jar};
use crate::nonce::{Nonce, NonceGenerator};
use crate::runtime::Runtime;
use crate::views::{Views, ViewsOf};
use crate::Database;

pub fn views<Db: ?Sized + Database>(db: &Db) -> &Views {
    DatabaseGen::views(db)
}

/// Salsa database methods whose implementation is generated by
/// the `#[salsa::database]` procedural macro.
///
/// # Safety
///
/// This trait is meant to be implemented by our procedural macro.
/// We need to document any non-obvious conditions that it satisfies.
pub unsafe trait DatabaseGen: Any {
    /// Upcast to a `dyn Database`.
    ///
    /// Only required because upcasts not yet stabilized (*grr*).
    ///
    /// # Safety
    ///
    /// Returns the same data pointer as `self`.
    fn as_salsa_database(&self) -> &dyn Database;

    /// Upcast to a `dyn Database`.
    ///
    /// Only required because upcasts not yet stabilized (*grr*).
    ///
    /// # Safety
    ///
    /// Returns the same data pointer as `self`.
    fn as_salsa_database_mut(&mut self) -> &mut dyn Database;

    /// Upcast to a `dyn DatabaseGen`.
    ///
    /// Only required because upcasts not yet stabilized (*grr*).
    ///
    /// # Safety
    ///
    /// Returns the same data pointer as `self`.
    fn as_salsa_database_gen(&self) -> &dyn DatabaseGen;

    /// Returns a reference to the underlying.
    fn views(&self) -> &Views;

    /// Returns the nonce for the underyling storage.
    ///
    /// # Safety
    ///
    /// This nonce is guaranteed to be unique for the database and never to be reused.
    fn nonce(&self) -> Nonce<StorageNonce>;

    /// Lookup the index assigned to the given jar (if any). This lookup is based purely on the jar's type.
    fn lookup_jar_by_type(&self, jar: &dyn Jar) -> Option<IngredientIndex>;

    /// Adds a jar to the database, returning the index of the first ingredient.
    /// If a jar of this type is already present, returns the existing index.
    fn add_or_lookup_jar_by_type(&self, jar: &dyn Jar) -> IngredientIndex;

    /// Gets an `&`-ref to an ingredient by index
    fn lookup_ingredient(&self, index: IngredientIndex) -> &dyn Ingredient;

    /// Gets an `&mut`-ref to an ingredient by index; also returns the runtime for further use
    fn lookup_ingredient_mut(
        &mut self,
        index: IngredientIndex,
    ) -> (&mut dyn Ingredient, &mut Runtime);

    /// Gets the salsa runtime
    fn runtime(&self) -> &Runtime;

    /// Gets the salsa runtime
    fn runtime_mut(&mut self) -> &mut Runtime;
}

/// This is the *actual* trait that the macro generates.
/// It simply gives access to the internal storage.
/// Note that it is NOT a supertrait of `Database`
/// because it is not `dyn`-safe.
///
/// # Safety
///
/// The `storage` field must be an owned field of
/// the implementing struct.
pub unsafe trait HasStorage: Database + Sized + Any {
    fn storage(&self) -> &Storage<Self>;
    fn storage_mut(&mut self) -> &mut Storage<Self>;
}

unsafe impl<T: HasStorage> DatabaseGen for T {
    fn as_salsa_database(&self) -> &dyn Database {
        self
    }

    fn as_salsa_database_mut(&mut self) -> &mut dyn Database {
        self
    }

    fn as_salsa_database_gen(&self) -> &dyn DatabaseGen {
        self
    }

    fn views(&self) -> &Views {
        &self.storage().shared.upcasts
    }

    fn nonce(&self) -> Nonce<StorageNonce> {
        self.storage().shared.nonce
    }

    fn lookup_jar_by_type(&self, jar: &dyn Jar) -> Option<IngredientIndex> {
        self.storage().lookup_jar_by_type(jar)
    }

    fn add_or_lookup_jar_by_type(&self, jar: &dyn Jar) -> IngredientIndex {
        self.storage().add_or_lookup_jar_by_type(jar)
    }

    fn lookup_ingredient(&self, index: IngredientIndex) -> &dyn Ingredient {
        self.storage().lookup_ingredient(index)
    }

    fn runtime(&self) -> &Runtime {
        &self.storage().runtime
    }

    fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.storage_mut().runtime
    }

    fn lookup_ingredient_mut(
        &mut self,
        index: IngredientIndex,
    ) -> (&mut dyn Ingredient, &mut Runtime) {
        self.storage_mut().lookup_ingredient_mut(index)
    }
}

impl dyn Database {
    /// Upcasts `self` to the given view.
    ///
    /// # Panics
    ///
    /// If the view has not been added to the database (see [`DatabaseView`][])
    #[track_caller]
    pub fn as_view<DbView: ?Sized + Database>(&self) -> &DbView {
        self.views().try_view_as(self).unwrap()
    }

    /// Upcasts `self` to the given view.
    ///
    /// # Panics
    ///
    /// If the view has not been added to the database (see [`DatabaseView`][])
    pub fn as_view_mut<DbView: ?Sized + Database>(&mut self) -> &mut DbView {
        // Avoid a borrow check error by cloning. This is the "uncommon" path so it seems fine.
        let upcasts = self.views().clone();
        upcasts.try_view_as_mut(self).unwrap()
    }
}

/// Nonce type representing the underlying database storage.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StorageNonce;

/// Generator for storage nonces.
static NONCE: NonceGenerator<StorageNonce> = NonceGenerator::new();

/// An ingredient index identifies a particular [`Ingredient`] in the database.
/// The database contains a number of jars, and each jar contains a number of ingredients.
/// Each ingredient is given a unique index as the database is being created.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct IngredientIndex(u32);

impl IngredientIndex {
    /// Create an ingredient index from a usize.
    pub(crate) fn from(v: usize) -> Self {
        assert!(v < (u32::MAX as usize));
        Self(v as u32)
    }

    /// Convert the ingredient index back into a usize.
    pub(crate) fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn cycle_recovery_strategy(self, db: &dyn Database) -> CycleRecoveryStrategy {
        db.lookup_ingredient(self).cycle_recovery_strategy()
    }

    pub fn successor(self, index: usize) -> Self {
        IngredientIndex(self.0 + 1 + index as u32)
    }
}

/// The "storage" struct stores all the data for the jars.
/// It is shared between the main database and any active snapshots.
pub struct Storage<Db: Database> {
    /// Data shared across all databases. This contains the ingredients needed by each jar.
    /// See the ["jars and ingredients" chapter](https://salsa-rs.github.io/salsa/plumbing/jars_and_ingredients.html)
    /// for more detailed description.
    shared: Shared<Db>,

    /// The runtime for this particular salsa database handle.
    /// Each handle gets its own runtime, but the runtimes have shared state between them.
    runtime: Runtime,
}

/// Data shared between all threads.
/// This is where the actual data for tracked functions, structs, inputs, etc lives,
/// along with some coordination variables between treads.
struct Shared<Db: Database> {
    upcasts: ViewsOf<Db>,

    nonce: Nonce<StorageNonce>,

    /// Map from the type-id of an `impl Jar` to the index of its first ingredient.
    /// This is using a `Mutex<FxHashMap>` (versus, say, a `FxDashMap`)
    /// so that we can protect `ingredients_vec` as well and predict what the
    /// first ingredient index will be. This allows ingredients to store their own indices.
    /// This may be worth refactoring in the future because it naturally adds more overhead to
    /// adding new kinds of ingredients.
    jar_map: Mutex<FxHashMap<TypeId, IngredientIndex>>,

    /// Vector of ingredients.
    ///
    /// Immutable unless the mutex on `ingredients_map` is held.
    ingredients_vec: ConcurrentVec<Box<dyn Ingredient>>,

    /// Indices of ingredients that require reset when a new revision starts.
    ingredients_requiring_reset: ConcurrentVec<IngredientIndex>,
}

// ANCHOR: default
impl<Db: Database> Default for Storage<Db> {
    fn default() -> Self {
        Self {
            shared: Shared {
                upcasts: Default::default(),
                nonce: NONCE.nonce(),
                jar_map: Default::default(),
                ingredients_vec: Default::default(),
                ingredients_requiring_reset: Default::default(),
            },
            runtime: Runtime::default(),
        }
    }
}
// ANCHOR_END: default

impl<Db: Database> Storage<Db> {
    /// Add an upcast function to type `T`.
    pub fn add_upcast<T: ?Sized + Any>(
        &mut self,
        func: fn(&Db) -> &T,
        func_mut: fn(&mut Db) -> &mut T,
    ) {
        self.shared.upcasts.add::<T>(func, func_mut)
    }

    /// Adds the ingredients in `jar` to the database if not already present.
    /// If a jar of this type is already present, returns the index.
    fn add_or_lookup_jar_by_type(&self, jar: &dyn Jar) -> IngredientIndex {
        let jar_type_id = jar.type_id();
        let mut jar_map = self.shared.jar_map.lock();
        *jar_map
        .entry(jar_type_id)
        .or_insert_with(|| {
            let index = IngredientIndex::from(self.shared.ingredients_vec.len());
            let ingredients = jar.create_ingredients(index);
            for ingredient in ingredients {
                let expected_index = ingredient.ingredient_index();

                if ingredient.requires_reset_for_new_revision() {
                    self.shared.ingredients_requiring_reset.push(expected_index);
                }

                let actual_index = self
                    .shared
                    .ingredients_vec
                    .push(ingredient);
                assert_eq!(
                    expected_index.as_usize(),
                    actual_index,
                    "ingredient `{:?}` was predicted to have index `{:?}` but actually has index `{:?}`",
                    self.shared.ingredients_vec.get(actual_index).unwrap(),
                    expected_index,
                    actual_index,
                );

            }
            index
        })
    }

    /// Return the index of the 1st ingredient from the given jar.
    pub fn lookup_jar_by_type(&self, jar: &dyn Jar) -> Option<IngredientIndex> {
        self.shared.jar_map.lock().get(&jar.type_id()).copied()
    }

    pub fn lookup_ingredient(&self, index: IngredientIndex) -> &dyn Ingredient {
        &**self.shared.ingredients_vec.get(index.as_usize()).unwrap()
    }

    fn lookup_ingredient_mut(
        &mut self,
        index: IngredientIndex,
    ) -> (&mut dyn Ingredient, &mut Runtime) {
        self.runtime.new_revision();

        for index in self.shared.ingredients_requiring_reset.iter() {
            self.shared
                .ingredients_vec
                .get_mut(index.as_usize())
                .unwrap()
                .reset_for_new_revision();
        }

        (
            &mut **self
                .shared
                .ingredients_vec
                .get_mut(index.as_usize())
                .unwrap(),
            &mut self.runtime,
        )
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }
}

/// Caches a pointer to an ingredient in a database.
/// Optimized for the case of a single database.
pub struct IngredientCache<I>
where
    I: Ingredient,
{
    cached_data: std::sync::OnceLock<(Nonce<StorageNonce>, *const I)>,
}

unsafe impl<I> Sync for IngredientCache<I> where I: Ingredient + Sync {}

impl<I> Default for IngredientCache<I>
where
    I: Ingredient,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I> IngredientCache<I>
where
    I: Ingredient,
{
    /// Create a new cache
    pub const fn new() -> Self {
        Self {
            cached_data: std::sync::OnceLock::new(),
        }
    }

    /// Get a reference to the ingredient in the database.
    /// If the ingredient is not already in the cache, it will be created.
    pub fn get_or_create<'s>(
        &self,
        storage: &'s dyn Database,
        create_index: impl Fn() -> IngredientIndex,
    ) -> &'s I {
        let &(nonce, ingredient) = self.cached_data.get_or_init(|| {
            let ingredient = self.create_ingredient(storage, &create_index);
            (storage.nonce(), ingredient as *const I)
        });

        if storage.nonce() == nonce {
            unsafe { &*ingredient }
        } else {
            self.create_ingredient(storage, &create_index)
        }
    }

    fn create_ingredient<'s>(
        &self,
        storage: &'s dyn Database,
        create_index: &impl Fn() -> IngredientIndex,
    ) -> &'s I {
        let index = create_index();
        storage.lookup_ingredient(index).assert_type::<I>()
    }
}
