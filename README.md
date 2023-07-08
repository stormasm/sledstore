
### Branch history

* testdb kicked things off

* suite drastically reduced the code in [suite.rs](https://github.com/datafuselabs/openraft/blob/main/openraft/src/testing/suite.rs) to just the test *get_log_entries*

* moretests add back in the test *get_initial_state_with_state* so we can start to exercise the state machine as well and applying entries to the state machine.

This enables me to better understand how all of this code is wired together...
