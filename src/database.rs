/*!
SQLite 数据库操作接口。

主框架中集成了 SQLite(通过 [`rusqlite`]),并将数据库操作接口传递给插件。插件**不直接**
持有连接,而是通过 [`DatabaseExt`] trait 读写数据库,包括表的创建、初始化、卸载清理。

宿主侧通过 [`SqliteDatabase`] 实现 [`DatabaseExt`] 并持有连接,插件管理器在调用生命周期
钩子时把 `&dyn DatabaseExt` 传入。

# 设计规约(参考 k8m `dao.DB()`)

* 表名必须包含插件名前缀,如 `demo_items`、`afp_articles`,避免命名冲突
* 所有数据库操作必须**幂等**(可重复执行)
* `init` / `upgrade` 用于安装与升级
* `drop` 用于卸载清理,配合 `keep_data` 选项决定是否保留数据
* 插件**不得**跨模块访问其他插件的表

# 示例

```ignore
use plugkit::database::DatabaseExt;

fn on_install(db: &dyn DatabaseExt) -> plugkit::error::Result<()> {
    db.execute(r#"
        CREATE TABLE IF NOT EXISTS demo_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT
        )
    "#)?;
    Ok(())
}

fn on_uninstall(db: &dyn DatabaseExt, keep_data: bool) -> plugkit::error::Result<()> {
    if !keep_data {
        db.execute("DROP TABLE IF EXISTS demo_items")?;
    }
    Ok(())
}
```

*/

// ------------------------------------------------------------------------------------------------
// 模块依赖
// ------------------------------------------------------------------------------------------------

use crate::error::{Error, ErrorKind, Result};
use parking_lot::Mutex;
use rusqlite::Connection;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;

// ------------------------------------------------------------------------------------------------
// 公开类型
// ------------------------------------------------------------------------------------------------

///
/// 插件可用的数据库操作接口。
///
/// 宿主实现此 trait 并在生命周期钩子中传递给插件。插件通过它读写数据库,
/// 包括表的创建、初始化、卸载清理。所有操作必须**幂等**。
///
/// **注意**:此 trait 仅暴露插件应当拥有的能力(执行 DDL/DML、查询),
/// 不暴露事务管理或连接获取——这些由宿主统一管理。
///
pub trait DatabaseExt: Send + Sync + Debug {
    /// 执行一条 SQL 语句(DDL 或 DML),返回受影响的行数。
    fn execute(&self, sql: &str) -> Result<usize>;

    /// 执行一条带参数的 SQL 语句(DDL 或 DML),返回受影响的行数。
    fn execute_with(&self, sql: &str, params: &[DbValue]) -> Result<usize>;

    /// 查询并返回行列表(每行是一个 `DbValue` 数组)。
    fn query(&self, sql: &str) -> Result<Vec<Vec<DbValue>>>;

    /// 查询并返回行列表(带参数)。
    fn query_with(&self, sql: &str, params: &[DbValue]) -> Result<Vec<Vec<DbValue>>>;

    /// 检查指定表是否存在。
    fn has_table(&self, table: &str) -> Result<bool>;

    /// 删除指定表(若存在)。卸载清理时调用。
    fn drop_table(&self, table: &str) -> Result<()>;

    /// 校验表名是否安全(仅允许 ASCII 字母数字、下划线、点)。
    /// 在执行任何将表名插值到 SQL 的操作(如 CREATE/DROP)前调用,防止 SQL 注入。
    fn validate_table_name(&self, table: &str) -> Result<()> {
        if table
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            Ok(())
        } else {
            Err(Error::from(ErrorKind::DatabaseError(format!(
                "invalid table name: {}",
                table
            ))))
        }
    }

    /// 返回数据库连接的简短描述(用于日志)。
    fn describe(&self) -> String;
}

///
/// 数据库值类型,用于参数与查询结果。支持 SQLite 的原生类型。
///
#[derive(Debug, Clone)]
pub enum DbValue {
    /// NULL。
    Null,
    /// 整数(i64)。
    Int(i64),
    /// 浮点数(f64)。
    Real(f64),
    /// 文本。
    Text(String),
    /// 二进制。
    Blob(Vec<u8>),
}

impl DbValue {
    /// 创建一个 NULL 值。
    pub fn null() -> Self {
        Self::Null
    }
    /// 创建一个整数值。
    pub fn int(v: i64) -> Self {
        Self::Int(v)
    }
    /// 创建一个浮点值。
    pub fn real(v: f64) -> Self {
        Self::Real(v)
    }
    /// 创建一个文本值。
    pub fn text<S: Into<String>>(v: S) -> Self {
        Self::Text(v.into())
    }
    /// 创建一个二进制值。
    pub fn blob(v: Vec<u8>) -> Self {
        Self::Blob(v)
    }
}

// ------------------------------------------------------------------------------------------------
// SqliteDatabase 实现
// ------------------------------------------------------------------------------------------------

///
/// 宿主侧 SQLite 数据库实现,持有连接并以 `parking_lot::Mutex` 保护并发。
///
/// 通过 [`SqliteDatabase::open`] 或 [`SqliteDatabase::in_memory`] 创建,
/// 传递给插件时用 `Arc<dyn DatabaseExt>` 共享。
///
#[derive(Debug)]
pub struct SqliteDatabase {
    connection: Arc<Mutex<Connection>>,
    descriptor: String,
}

impl SqliteDatabase {
    ///
    /// 打开(或创建)一个 SQLite 数据库文件。
    ///
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path.as_ref()).map_err(|e| {
            Error::from(ErrorKind::DatabaseError(format!(
                "open {:?}: {}",
                path.as_ref(),
                e
            )))
        })?;
        // 启用外键约束与 WAL,提升并发与一致性
        let _ = conn.pragma_update(None, "foreign_keys", "ON");
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let descriptor = path.as_ref().to_string_lossy().to_string();
        Ok(Self {
            connection: Arc::new(Mutex::new(conn)),
            descriptor,
        })
    }

    ///
    /// 创建一个内存中的 SQLite 数据库(用于测试与演示)。
    ///
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| Error::from(ErrorKind::DatabaseError(format!("in_memory: {}", e))))?;
        let _ = conn.pragma_update(None, "foreign_keys", "ON");
        Ok(Self {
            connection: Arc::new(Mutex::new(conn)),
            descriptor: ":memory:".to_string(),
        })
    }

    /// 返回共享的连接 Arc,便于宿主直接执行复杂查询(插件不使用)。
    pub fn shared_connection(&self) -> Arc<Mutex<Connection>> {
        self.connection.clone()
    }
}

impl DatabaseExt for SqliteDatabase {
    fn execute(&self, sql: &str) -> Result<usize> {
        let conn = self.connection.lock();
        let affected = conn
            .execute(sql, [])
            .map_err(|e| Error::from(ErrorKind::DatabaseError(format!("execute: {}", e))))?;
        Ok(affected)
    }

    fn execute_with(&self, sql: &str, params: &[DbValue]) -> Result<usize> {
        let conn = self.connection.lock();
        // rusqlite 的 Params 通过 &[&dyn ToSql] 实现,故收集引用
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let affected = conn
            .execute(sql, rusqlite_params.as_slice())
            .map_err(|e| Error::from(ErrorKind::DatabaseError(format!("execute_with: {}", e))))?;
        Ok(affected)
    }

    fn query(&self, sql: &str) -> Result<Vec<Vec<DbValue>>> {
        let conn = self.connection.lock();
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| Error::from(ErrorKind::DatabaseError(format!("prepare: {}", e))))?;
        let rows: Vec<Vec<DbValue>> = stmt
            .query_map([], |row| {
                let count = row.as_ref().column_count();
                let mut out = Vec::with_capacity(count);
                for i in 0..count {
                    out.push(read_column(row, i)?);
                }
                Ok(out)
            })
            .map_err(|e| Error::from(ErrorKind::DatabaseError(format!("query: {}", e))))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    fn query_with(&self, sql: &str, params: &[DbValue]) -> Result<Vec<Vec<DbValue>>> {
        let conn = self.connection.lock();
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| Error::from(ErrorKind::DatabaseError(format!("prepare: {}", e))))?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        let rows: Vec<Vec<DbValue>> = stmt
            .query_map(rusqlite_params.as_slice(), |row| {
                let count = row.as_ref().column_count();
                let mut out = Vec::with_capacity(count);
                for i in 0..count {
                    out.push(read_column(row, i)?);
                }
                Ok(out)
            })
            .map_err(|e| Error::from(ErrorKind::DatabaseError(format!("query_with: {}", e))))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    fn has_table(&self, table: &str) -> Result<bool> {
        let conn = self.connection.lock();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)",
                [table],
                |row| row.get(0),
            )
            .map_err(|e| Error::from(ErrorKind::DatabaseError(format!("has_table: {}", e))))?;
        Ok(exists)
    }

    fn drop_table(&self, table: &str) -> Result<()> {
        // 复用 trait 默认实现 validate_table_name,防止 SQL 注入
        self.validate_table_name(table)?;
        let sql = format!("DROP TABLE IF EXISTS {}", table);
        let conn = self.connection.lock();
        conn.execute(&sql, [])
            .map_err(|e| Error::from(ErrorKind::DatabaseError(format!("drop_table: {}", e))))?;
        Ok(())
    }

    fn describe(&self) -> String {
        format!("sqlite://{}", self.descriptor)
    }
}

// ------------------------------------------------------------------------------------------------
// 私有函数
// ------------------------------------------------------------------------------------------------

fn read_column(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<DbValue> {
    let value = row.get_ref(idx)?;
    Ok(match value {
        rusqlite::types::ValueRef::Null => DbValue::Null,
        rusqlite::types::ValueRef::Integer(i) => DbValue::Int(i),
        rusqlite::types::ValueRef::Real(f) => DbValue::Real(f),
        rusqlite::types::ValueRef::Text(s) => DbValue::Text(String::from_utf8_lossy(s).to_string()),
        rusqlite::types::ValueRef::Blob(b) => DbValue::Blob(b.to_vec()),
    })
}

// DbValue 实现 rusqlite::ToSql,使其可作为绑定参数使用
impl rusqlite::ToSql for DbValue {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            DbValue::Null => rusqlite::types::Null.to_sql(),
            DbValue::Int(i) => i.to_sql(),
            DbValue::Real(f) => f.to_sql(),
            DbValue::Text(s) => s.to_sql(),
            DbValue::Blob(b) => b.to_sql(),
        }
    }
}

// ------------------------------------------------------------------------------------------------
// 单元测试
// ------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_create_and_query() {
        let db = SqliteDatabase::in_memory().unwrap();
        db.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();
        assert!(db.has_table("t").unwrap());
        db.execute("INSERT INTO t (id, name) VALUES (1, 'hello')")
            .unwrap();
        let rows = db.query("SELECT id, name FROM t").unwrap();
        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0][0], DbValue::Int(1)));
        assert!(matches!(rows[0][1].clone(), DbValue::Text(_)));
    }

    #[test]
    fn test_query_with_params() {
        let db = SqliteDatabase::in_memory().unwrap();
        db.execute("CREATE TABLE t (id INTEGER, name TEXT)")
            .unwrap();
        db.execute_with(
            "INSERT INTO t (id, name) VALUES (?, ?)",
            &[DbValue::int(1), DbValue::text("a")],
        )
        .unwrap();
        db.execute_with(
            "INSERT INTO t (id, name) VALUES (?, ?)",
            &[DbValue::int(2), DbValue::text("b")],
        )
        .unwrap();
        let rows = db
            .query_with("SELECT name FROM t WHERE id = ?", &[DbValue::int(2)])
            .unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_drop_table() {
        let db = SqliteDatabase::in_memory().unwrap();
        db.execute("CREATE TABLE t (id INTEGER)").unwrap();
        assert!(db.has_table("t").unwrap());
        db.drop_table("t").unwrap();
        assert!(!db.has_table("t").unwrap());
    }

    #[test]
    fn test_drop_table_rejects_injection() {
        let db = SqliteDatabase::in_memory().unwrap();
        let err = db.drop_table("t; DROP TABLE x;--").unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::DatabaseError(_)));
    }
}
