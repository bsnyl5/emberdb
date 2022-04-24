use crate::{
    sql::{
        insert::InsertPlan,
        join::{
            grace::{GraceHashJoinOp, GraceHashJoinPlan, PartitionedQueue},
            queue::MemoryAllocator,
        },
        scan::SeqScanPlan,
        tx::Txn,
        util::RawInput,
        DataBlock, SqlResult,
    },
    storage::sled::Sled,
};
use async_trait::async_trait;
use datafusion::arrow::datatypes::{Schema, SchemaRef};
use futures::{stream::Stream, Future};
use std::{env::temp_dir, fmt::Debug, pin::Pin, rc::Rc, sync::Arc};
use tempfile::{tempfile, NamedTempFile};

#[derive(Clone)]
pub struct ExecutionContext {
    storage: Arc<dyn Storage>,
    // bpm: Rc<BufferPoolManager>,
    pub txn: Txn,
    queue: Arc<MemoryAllocator>, // TODO: make this into a trait object
}

impl ExecutionContext {
    pub fn new_for_test() -> Self {
        let inmem = MemoryAllocator::new();
        // let temp = tempfile().expect();
        let file = NamedTempFile::new().expect("failed creating temp file");
        let path = file.into_temp_path().to_str().unwrap().to_string();
        let db = Sled::new(path).expect("failed creating sled");

        ExecutionContext {
            txn: Txn {},
            storage: Arc::new(db),
            queue: Arc::new(inmem),
        }
    }
    pub fn new(store: Arc<dyn Storage>) -> Self {
        let inmem = MemoryAllocator::new();
        ExecutionContext {
            txn: Txn {},
            storage: store,
            queue: Arc::new(inmem),
        }
    }
    pub fn get_txn(&self) -> &Txn {
        &self.txn
    }

    pub fn get_storage(&self) -> Arc<dyn Storage> {
        self.storage.clone()
    }

    pub fn new_queue(&self, schema: SchemaRef) -> Arc<dyn PartitionedQueue> {
        (*self.queue).alloc(schema)
    }
}

/// Trait for types that stream [arrow::record_batch::RecordBatch]
pub trait DataBlockStream: Stream<Item = SqlResult<DataBlock>> {
    fn schema(self) -> SchemaRef;
}

/// Trait for types that stream [arrow::record_batch::RecordBatch]
pub type SendableDataBlockStream = Pin<Box<dyn DataBlockStream + Send + Sync>>;

pub type SendableResult = Pin<Box<dyn Future<Output = SqlResult<()>> + Send + Sync>>;
pub struct SchemaStream {
    inner: Pin<Box<dyn Stream<Item = SqlResult<DataBlock>> + Send + Sync>>,
    schema: SchemaRef,
}

unsafe impl Send for SchemaStream {}
unsafe impl Sync for SchemaStream {}
impl SchemaStream {
    pub fn new(
        schema: SchemaRef,
        inner: Pin<Box<dyn Stream<Item = SqlResult<DataBlock>> + Send + Sync>>,
    ) -> Pin<Box<Self>> {
        Box::pin(SchemaStream { inner, schema })
    }
}
impl DataBlockStream for SchemaStream {
    fn schema(self) -> SchemaRef {
        self.schema.clone()
    }
}

impl Stream for SchemaStream {
    type Item = SqlResult<DataBlock>;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[async_trait]
pub trait Operator: Debug + Send + Sync {
    async fn execute(&mut self, ctx: ExecutionContext) -> SqlResult<SendableDataBlockStream>;
    fn execute_sync(&mut self, ctx: ExecutionContext) -> SqlResult<SendableDataBlockStream>;

    fn schema(&self) -> SchemaRef;
}

pub enum PlanType {
    SeqScan(SeqScanPlan),
    RawInput(RawInput),
    Insert(InsertPlan),
    GraceHashJoin(GraceHashJoinPlan),
    IndexScan,
    Update,
    Delete,
    Aggregation,
    Limit,
    HashJoin,
}

pub struct Executor {}
impl Executor {
    // TODO: maybe return some async iter like stream in the future
    pub async fn execute(
        plan: PlanType,
        ctx: ExecutionContext,
    ) -> SqlResult<SendableDataBlockStream> {
        let mut operator: Box<dyn Operator> = Self::create_operator(plan, ctx.clone());
        operator.execute(ctx).await

        /* let mut ret = vec![];
        while let Some(value) = retstream.next().await {
            let some_block: DataBlock = value?;
            ret.extend(some_block.columns);

            println!("{}", value);
        }

        // TODO: not sure if this calls next() for non_result operator

        Ok(DataBlock::new(retstream.schema(), ret)) */
    }

    pub fn create_operator(plan_type: PlanType, ctx: ExecutionContext) -> Box<dyn Operator> {
        match plan_type {
            /* PlanType::SeqScan(plan) => {
                Box::new(SeqScanner::from_plan(plan, ctx.clone())) as Box<dyn Operator>
            }
            PlanType::Insert(plan) => Box::new(Insert::new(plan, ctx.clone())),
            PlanType::RawInput(raw) => Box::new(raw), */
            PlanType::GraceHashJoin(plan) => {
                let cloned = ctx.clone();
                Box::new(GraceHashJoinOp::from_plan(
                    plan,
                    // move || -> Rc<dyn PartitionedQueue> { cloned.new_queue() },
                    ctx.clone(),
                ))
            }
            _ => {
                todo!("todo")
            }
        }
    }
    pub fn create_from_subplan_operator(
        plan_type: PlanType,
        ctx: ExecutionContext,
    ) -> Box<dyn Operator> {
        match plan_type {
            /* PlanType::SeqScan(plan) => {
                Box::new(SeqScanner::from_plan(plan, ctx)) as Box<dyn Operator>
            }
            PlanType::RawInput(raw) => Box::new(raw), */
            _ => {
                todo!("todo")
            }
        }
    }
}
pub trait Catalog {
    /* fn create_table(&self, tablename: String, schema: Schema) -> SqlResult<TableMeta>;

    fn get_table(&self, tablename: String) -> SqlResult<TableMeta>; */
}

pub type RID = u64;

#[async_trait]
pub trait Storage: Catalog + Sync + Send {
    // TODO: batch insert
    async fn insert_tuples(
        &self,
        table: &str,
        data: SendableDataBlockStream,
        txn: &Txn,
    ) -> SqlResult<SendableDataBlockStream>;
    async fn delete(&self, table: &str, data: SendableDataBlockStream, txn: &Txn) -> SqlResult<()>;
    async fn get_tuples(
        &self,
        table: &str,
        rids: SendableDataBlockStream,
        txn: &Txn,
    ) -> SqlResult<SendableDataBlockStream>;

    async fn scan(&self, table: &str, txn: &Txn) -> SqlResult<SendableDataBlockStream>;
}

#[cfg(test)]
pub mod tests {
    use super::Storage;
    use crate::{
        sql::{
            exe::{Executor, PlanType},
            scan::SeqScanPlan,
            table_gen::GenTableUtil,
            ExecutionContext,
        },
        storage::sled::Sled,
    };
    use std::{env::current_dir, rc::Rc, sync::Arc};
    use tempfile::NamedTempFile;

    #[test]
    fn test_seq_scan() {}

    pub struct Dependencies {
        storage: Arc<dyn Storage>,
        executor: Executor,
        gen_table: GenTableUtil,
    }

    pub fn setup() -> Dependencies {
        let file = NamedTempFile::new().expect("failed creating temp file");
        let path = file.into_temp_path().to_str().unwrap().to_string();
        let db = Sled::new(path).expect("failed creating sled");
        let cloned = Arc::new(db);
        Dependencies {
            storage: cloned.clone(),
            executor: Executor {},
            gen_table: GenTableUtil {
                ctx: ExecutionContext::new(cloned),
            },
        }
    }
}