use rbatis::RBatis;
use rbdc_mysql::MysqlDriver;
use std::sync::Arc;
use lazy_static::lazy_static;

// 全局 RBATIS 实例
lazy_static! {
    pub static ref RBATIS_POOL: Arc<RBatis> = {
        let rb = RBatis::new();
        // 初始化数据库连接池
        // 注意：这里需要根据你的实际数据库配置进行修改
        rb.init(MysqlDriver {}, "mysql://admin:b7371d927aec647d@172.17.0.185:3306/grave").unwrap();
        Arc::new(rb)
    };
}