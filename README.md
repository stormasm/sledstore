
### Branch history

* testdb kicked things off

* suite drastically reduced the code in [suite.rs](https://github.com/datafuselabs/openraft/blob/main/openraft/src/testing/suite.rs) to just the test *get_log_entries*

* moretests add back in the test *get_initial_state_with_state* so we can start to exercise the state machine as well and applying entries to the state machine.

This enables me to better understand how all of this code is wired together...

### Note about running tests

In these tests I am creating a real *db*

```rust
let td = PathBuf::from(r"./db");
let db: sled::Db = sled::open(td.as_path()).unwrap();
```

instead of a temporary *db*

```rust
let td = TempDir::new().expect("couldn't create temp dir");
let db: sled::Db = sled::open(td.path()).unwrap();
```

just so I can see what is going on with the logs and state machine...

For that reason sometimes it is necessary to delete the *db* that gets created

Also you can only have one test at a time being run and if there are any issues
with the tests failing you must make sure only one test is being run.

So you must comment out the other tests...

```rust
//run_fut(run_test(builder, Self::get_log_entries))?;
run_fut(run_test(builder, Self::get_initial_state_with_state))?;
```
