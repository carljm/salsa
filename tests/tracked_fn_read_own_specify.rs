use expect_test::expect;
use salsa::Database as SalsaDatabase;
mod common;
use common::{HasLogger, Logger};

#[salsa::db]
trait Db: salsa::Database + HasLogger {}

#[salsa::input]
struct MyInput {
    field: u32,
}

#[salsa::tracked]
struct MyTracked<'db> {
    field: u32,
}

#[salsa::tracked]
fn tracked_fn<'db>(db: &'db dyn Db, input: MyInput) -> u32 {
    db.push_log(format!("tracked_fn({input:?})"));
    let t = MyTracked::new(db, input.field(db) * 2);
    tracked_fn_extra::specify(db, t, 2222);
    tracked_fn_extra(db, t)
}

#[salsa::tracked(specify)]
fn tracked_fn_extra<'db>(db: &dyn Db, input: MyTracked<'db>) -> u32 {
    db.push_log(format!("tracked_fn_extra({input:?})"));
    0
}

#[salsa::db]
#[derive(Default)]
struct Database {
    storage: salsa::Storage<Self>,
    logger: Logger,
}

#[salsa::db]
impl salsa::Database for Database {}

#[salsa::db]
impl Db for Database {}

impl HasLogger for Database {
    fn logger(&self) -> &Logger {
        &self.logger
    }
}

#[test]
fn execute() {
    let mut db = Database::default();
    let input = MyInput::new(&db, 22);
    assert_eq!(tracked_fn(&db, input), 2222);
    db.assert_logs(expect![[r#"
        [
            "tracked_fn(MyInput { [salsa id]: 0, field: 22 })",
        ]"#]]);

    // A "synthetic write" causes the system to act *as though* some
    // input of durability `durability` has changed.
    db.synthetic_write(salsa::Durability::LOW);

    // Re-run the query on the original input. Nothing re-executes!
    assert_eq!(tracked_fn(&db, input), 2222);
    db.assert_logs(expect!["[]"]);
}
