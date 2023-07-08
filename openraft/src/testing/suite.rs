use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;

use anyerror::AnyError;
use tokio::sync::oneshot;

use crate::entry::RaftEntry;
use crate::log_id::RaftLogId;
use crate::raft_state::LogStateReader;
use crate::storage::LogFlushed;
use crate::storage::RaftLogReaderExt;
use crate::storage::RaftLogStorage;
use crate::storage::RaftStateMachine;
use crate::storage::StorageHelper;
use crate::testing::StoreBuilder;
use crate::vote::CommittedLeaderId;
use crate::AppData;
use crate::AppDataResponse;
use crate::LogId;
use crate::NodeId;
use crate::OptionalSend;
use crate::RaftTypeConfig;
use crate::StorageError;
use crate::StorageIOError;
use crate::Vote;

const NODE_ID: u64 = 0;

/// Test suite to ensure a `RaftStore` impl works as expected.
pub struct Suite<C, LS, SM, B, G>
where
    C: RaftTypeConfig,
    C::D: AppData + Debug,
    C::R: AppDataResponse + Debug,
    LS: RaftLogStorage<C>,
    SM: RaftStateMachine<C>,
    B: StoreBuilder<C, LS, SM, G>,
    G: Send + Sync,
{
    _p: PhantomData<(C, LS, SM, B, G)>,
}

#[allow(unused)]
impl<C, LS, SM, B, G> Suite<C, LS, SM, B, G>
where
    C: RaftTypeConfig,
    C::D: AppData + Debug,
    C::R: AppDataResponse + Debug,
    C::NodeId: From<u64>,
    LS: RaftLogStorage<C>,
    SM: RaftStateMachine<C>,
    B: StoreBuilder<C, LS, SM, G>,
    G: Send + Sync,
{
    pub fn test_all(builder: B) -> Result<(), StorageError<C::NodeId>> {
        Suite::test_store(&builder)?;
        Ok(())
    }

    pub fn test_store(builder: &B) -> Result<(), StorageError<C::NodeId>> {
        run_fut(run_test(builder, Self::get_log_entries))?;
        run_fut(run_test(builder, Self::get_initial_state_with_state))?;
        Ok(())
    }

    pub async fn get_log_entries(mut store: LS, mut sm: SM) -> Result<(), StorageError<C::NodeId>> {
        Self::feed_10_logs_vote_self(&mut store).await?;

        tracing::info!("--- get start == stop");
        {
            let logs = store.get_log_entries(3..3).await?;
            assert_eq!(logs.len(), 0, "expected no logs to be returned");
        }

        tracing::info!("--- get start < stop");
        {
            let logs = store.get_log_entries(5..7).await?;

            assert_eq!(logs.len(), 2);
            assert_eq!(*logs[0].get_log_id(), log_id_0(1, 5));
            assert_eq!(*logs[1].get_log_id(), log_id_0(1, 6));
        }

        Ok(())
    }

    pub async fn get_initial_state_with_state(
        mut store: LS,
        mut sm: SM,
    ) -> Result<(), StorageError<C::NodeId>> {
        Self::default_vote(&mut store).await?;

        append(
            &mut store,
            [
                blank_ent_0::<C>(0, 0),
                blank_ent_0::<C>(1, 1),
                blank_ent_0::<C>(3, 2),
            ],
        )
        .await?;

        apply(&mut sm, [blank_ent_0::<C>(3, 1)]).await?;

        let initial = StorageHelper::new(&mut store, &mut sm)
            .get_initial_state()
            .await?;

        assert_eq!(
            Some(&log_id_0(3, 2)),
            initial.last_log_id(),
            "state machine has higher log"
        );
        assert_eq!(
            initial.committed(),
            Some(&log_id_0(3, 1)),
            "unexpected value for last applied log"
        );
        assert_eq!(
            Vote::new(1, NODE_ID.into()),
            *initial.vote_ref(),
            "unexpected value for default hard state"
        );
        Ok(())
    }

    pub async fn feed_10_logs_vote_self(sto: &mut LS) -> Result<(), StorageError<C::NodeId>> {
        append(sto, [blank_ent_0::<C>(0, 0)]).await?;

        for i in 1..=10 {
            append(sto, [blank_ent_0::<C>(1, i)]).await?;
        }

        Self::default_vote(sto).await?;

        Ok(())
    }

    pub async fn default_vote(sto: &mut LS) -> Result<(), StorageError<C::NodeId>> {
        sto.save_vote(&Vote::new(1, NODE_ID.into())).await?;

        Ok(())
    }
}

fn log_id_0<NID: NodeId>(term: u64, index: u64) -> LogId<NID>
where
    NID: From<u64>,
{
    LogId {
        leader_id: CommittedLeaderId::new(term, NODE_ID.into()),
        index,
    }
}

/// Create a blank log entry with node_id 0 for test.
fn blank_ent_0<C: RaftTypeConfig>(term: u64, index: u64) -> C::Entry
where
    C::NodeId: From<u64>,
{
    C::Entry::new_blank(log_id_0(term, index))
}

/// Block until a future is finished.
/// The future will be running in a clean tokio runtime, to prevent an unfinished task affecting the
/// test.
pub fn run_fut<NID, F>(f: F) -> Result<(), StorageError<NID>>
where
    NID: NodeId,
    F: Future<Output = Result<(), StorageError<NID>>>,
{
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(f)?;
    Ok(())
}

/// Build a `RaftStorage` implementation and run a test on it.
async fn run_test<C, LS, SM, G, B, TestFn, Ret, Fu>(
    builder: &B,
    test_fn: TestFn,
) -> Result<Ret, StorageError<C::NodeId>>
where
    C: RaftTypeConfig,
    LS: RaftLogStorage<C>,
    SM: RaftStateMachine<C>,
    B: StoreBuilder<C, LS, SM, G>,
    Fu: Future<Output = Result<Ret, StorageError<C::NodeId>>> + OptionalSend,
    TestFn: Fn(LS, SM) -> Fu + Sync + Send,
{
    let (_g, store, sm) = builder.build().await?;
    test_fn(store, sm).await
}

/// A wrapper for calling nonblocking `RaftStorage::apply_to_state_machine()`
async fn apply<C, SM, I>(sm: &mut SM, entries: I) -> Result<(), StorageError<C::NodeId>>
where
    C: RaftTypeConfig,
    SM: RaftStateMachine<C>,
    I: IntoIterator<Item = C::Entry> + OptionalSend,
    I::IntoIter: OptionalSend,
{
    sm.apply(entries).await?;
    Ok(())
}

/// A wrapper for calling nonblocking `RaftStorage::append_to_log()`
async fn append<C, LS, I>(store: &mut LS, entries: I) -> Result<(), StorageError<C::NodeId>>
where
    C: RaftTypeConfig,
    LS: RaftLogStorage<C>,
    I: IntoIterator<Item = C::Entry>,
{
    let entries = entries.into_iter().collect::<Vec<_>>();
    let last_log_id = *entries.last().unwrap().get_log_id();

    let (tx, rx) = oneshot::channel();

    let cb = LogFlushed::new(Some(last_log_id), tx);

    store.append(entries, cb).await?;
    rx.await
        .unwrap()
        .map_err(|e| StorageIOError::write_logs(AnyError::error(e)))?;
    Ok(())
}
