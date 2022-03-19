use crate::bpm::BufferPoolManager;
use crate::sql::tx::Txn;
use std::rc::Rc;

pub struct ExecutionContext {
    storage: Rc<dyn Storage>,
    bpm: Rc<BufferPoolManager>,
    txn: Txn,
}

impl ExecutionContext {
    pub fn get_txn(&self) -> &Txn {
        &self.txn
    }

    pub fn get_bpm(&self) -> Rc<BufferPoolManager> {
        self.bpm.clone()
    }

    pub fn get_storage(&self) -> Rc<dyn Storage> {
        self.storage.clone()
    }
}
pub trait Storage {
    fn create_table(&self) {}

    fn get_table(&self) {}
}
/* class ExecutorContext {
 public:
  /**
   * Creates an ExecutorContext for the transaction that is executing the query.
   * @param transaction the transaction executing the query
   * @param catalog the catalog that the executor should use
   * @param bpm the buffer pool manager that the executor should use
   * @param txn_mgr the transaction manager that the executor should use
   * @param lock_mgr the lock manager that the executor should use
   */
  ExecutorContext(Transaction *transaction, Catalog *catalog, BufferPoolManager *bpm, TransactionManager *txn_mgr,
                  LockManager *lock_mgr)
      : transaction_(transaction), catalog_{catalog}, bpm_{bpm}, txn_mgr_(txn_mgr), lock_mgr_(lock_mgr) {}

  DISALLOW_COPY_AND_MOVE(ExecutorContext);

  ~ExecutorContext() = default;

  /** @return the running transaction */
  Transaction *GetTransaction() const { return transaction_; }

  /** @return the catalog */
  Catalog *GetCatalog() { return catalog_; }

  /** @return the buffer pool manager */
  BufferPoolManager *GetBufferPoolManager() { return bpm_; }

  /** @return the log manager - don't worry about it for now */
  LogManager *GetLogManager() { return nullptr; }

  /** @return the lock manager */
  LockManager *GetLockManager() { return lock_mgr_; }

  /** @return the transaction manager */
  TransactionManager *GetTransactionManager() { return txn_mgr_; }

 private:
  Transaction *transaction_;
  Catalog *catalog_;
  BufferPoolManager *bpm_;
  TransactionManager *txn_mgr_;
  LockManager *lock_mgr_;
} */