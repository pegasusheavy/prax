//! Connection pool configuration.

use super::{ConnectionOptions, PoolOptions};
use std::time::Duration;
use tracing::info;

/// Complete pool configuration combining connection and pool options.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Connection options.
    pub connection: ConnectionOptions,
    /// Pool options.
    pub pool: PoolOptions,
    /// Number of retry attempts for failed connections.
    pub retry_attempts: u32,
    /// Delay between retry attempts.
    pub retry_delay: Duration,
    /// Health check interval.
    pub health_check_interval: Option<Duration>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            connection: ConnectionOptions::default(),
            pool: PoolOptions::default(),
            retry_attempts: 3,
            retry_delay: Duration::from_millis(500),
            health_check_interval: Some(Duration::from_secs(30)),
        }
    }
}

impl PoolConfig {
    /// Create a new pool configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set connection options.
    pub fn connection(mut self, options: ConnectionOptions) -> Self {
        self.connection = options;
        self
    }

    /// Set pool options.
    pub fn pool(mut self, options: PoolOptions) -> Self {
        self.pool = options;
        self
    }

    /// Set max connections.
    pub fn max_connections(mut self, n: u32) -> Self {
        self.pool.max_connections = n;
        self
    }

    /// Set min connections.
    pub fn min_connections(mut self, n: u32) -> Self {
        self.pool.min_connections = n;
        self
    }

    /// Set connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connection.connect_timeout = timeout;
        self
    }

    /// Set acquire timeout.
    pub fn acquire_timeout(mut self, timeout: Duration) -> Self {
        self.pool.acquire_timeout = timeout;
        self
    }

    /// Set idle timeout.
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.pool.idle_timeout = Some(timeout);
        self
    }

    /// Set max lifetime.
    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.pool.max_lifetime = Some(lifetime);
        self
    }

    /// Set retry attempts.
    pub fn retry_attempts(mut self, attempts: u32) -> Self {
        self.retry_attempts = attempts;
        self
    }

    /// Set retry delay.
    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Set health check interval.
    pub fn health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = Some(interval);
        self
    }

    /// Disable health checks.
    pub fn no_health_check(mut self) -> Self {
        self.health_check_interval = None;
        self
    }

    /// Create a configuration optimized for low-latency.
    pub fn low_latency() -> Self {
        info!(
            max_connections = 20,
            min_connections = 5,
            "PoolConfig::low_latency() initialized"
        );
        Self {
            connection: ConnectionOptions::new().connect_timeout(Duration::from_secs(5)),
            pool: PoolOptions::new()
                .max_connections(20)
                .min_connections(5)
                .acquire_timeout(Duration::from_secs(5))
                .idle_timeout(Duration::from_secs(60)),
            retry_attempts: 1,
            retry_delay: Duration::from_millis(100),
            health_check_interval: Some(Duration::from_secs(10)),
        }
    }

    /// Create a configuration optimized for high throughput.
    pub fn high_throughput() -> Self {
        info!(
            max_connections = 50,
            min_connections = 10,
            "PoolConfig::high_throughput() initialized"
        );
        Self {
            connection: ConnectionOptions::new().connect_timeout(Duration::from_secs(30)),
            pool: PoolOptions::new()
                .max_connections(50)
                .min_connections(10)
                .acquire_timeout(Duration::from_secs(30))
                .idle_timeout(Duration::from_secs(300)),
            retry_attempts: 3,
            retry_delay: Duration::from_secs(1),
            health_check_interval: Some(Duration::from_secs(60)),
        }
    }

    /// Create a configuration for development/testing.
    pub fn development() -> Self {
        info!(
            max_connections = 5,
            min_connections = 1,
            "PoolConfig::development() initialized"
        );
        Self {
            connection: ConnectionOptions::new().connect_timeout(Duration::from_secs(5)),
            pool: PoolOptions::new()
                .max_connections(5)
                .min_connections(1)
                .acquire_timeout(Duration::from_secs(5))
                .test_before_acquire(false),
            retry_attempts: 0,
            retry_delay: Duration::from_millis(0),
            health_check_interval: None,
        }
    }

    /// Create a configuration optimized for read-heavy workloads.
    ///
    /// Features:
    /// - More connections (reads can parallelize)
    /// - Longer connection lifetime (cached statement benefits)
    /// - Moderate health check interval
    pub fn read_heavy() -> Self {
        info!(
            max_connections = 30,
            min_connections = 5,
            "PoolConfig::read_heavy() initialized"
        );
        Self {
            connection: ConnectionOptions::new().connect_timeout(Duration::from_secs(10)),
            pool: PoolOptions::new()
                .max_connections(30)
                .min_connections(5)
                .acquire_timeout(Duration::from_secs(15))
                .idle_timeout(Duration::from_secs(300))
                .max_lifetime(Duration::from_secs(3600)), // 1 hour for cached statements
            retry_attempts: 2,
            retry_delay: Duration::from_millis(200),
            health_check_interval: Some(Duration::from_secs(30)),
        }
    }

    /// Create a configuration optimized for write-heavy workloads.
    ///
    /// Features:
    /// - Fewer connections (writes are serialized)
    /// - Shorter lifetime (avoid long-running transactions)
    /// - Frequent health checks
    pub fn write_heavy() -> Self {
        info!(
            max_connections = 15,
            min_connections = 3,
            "PoolConfig::write_heavy() initialized"
        );
        Self {
            connection: ConnectionOptions::new().connect_timeout(Duration::from_secs(10)),
            pool: PoolOptions::new()
                .max_connections(15)
                .min_connections(3)
                .acquire_timeout(Duration::from_secs(20))
                .idle_timeout(Duration::from_secs(120))
                .max_lifetime(Duration::from_secs(900)), // 15 minutes
            retry_attempts: 3,
            retry_delay: Duration::from_millis(500),
            health_check_interval: Some(Duration::from_secs(15)),
        }
    }

    /// Create a configuration optimized for mixed workloads.
    ///
    /// Balanced settings for applications with both reads and writes.
    pub fn mixed_workload() -> Self {
        info!(
            max_connections = 25,
            min_connections = 5,
            "PoolConfig::mixed_workload() initialized"
        );
        Self {
            connection: ConnectionOptions::new().connect_timeout(Duration::from_secs(10)),
            pool: PoolOptions::new()
                .max_connections(25)
                .min_connections(5)
                .acquire_timeout(Duration::from_secs(15))
                .idle_timeout(Duration::from_secs(180))
                .max_lifetime(Duration::from_secs(1800)), // 30 minutes
            retry_attempts: 2,
            retry_delay: Duration::from_millis(300),
            health_check_interval: Some(Duration::from_secs(30)),
        }
    }

    /// Create a configuration optimized for batch processing.
    ///
    /// Features:
    /// - Longer timeouts for batch operations
    /// - More connections for parallel batch processing
    /// - Infrequent health checks
    pub fn batch_processing() -> Self {
        info!(
            max_connections = 40,
            min_connections = 10,
            "PoolConfig::batch_processing() initialized"
        );
        Self {
            connection: ConnectionOptions::new().connect_timeout(Duration::from_secs(30)),
            pool: PoolOptions::new()
                .max_connections(40)
                .min_connections(10)
                .acquire_timeout(Duration::from_secs(60))
                .idle_timeout(Duration::from_secs(600))
                .max_lifetime(Duration::from_secs(7200)), // 2 hours
            retry_attempts: 5,
            retry_delay: Duration::from_secs(2),
            health_check_interval: Some(Duration::from_secs(120)),
        }
    }

    /// Create a configuration for serverless environments.
    ///
    /// Features:
    /// - Quick connection acquisition
    /// - Aggressive connection recycling
    /// - No minimum connections (cold start friendly)
    pub fn serverless() -> Self {
        info!(
            max_connections = 10,
            min_connections = 0,
            "PoolConfig::serverless() initialized"
        );
        Self {
            connection: ConnectionOptions::new().connect_timeout(Duration::from_secs(3)),
            pool: PoolOptions::new()
                .max_connections(10)
                .min_connections(0)
                .acquire_timeout(Duration::from_secs(3))
                .idle_timeout(Duration::from_secs(30))
                .max_lifetime(Duration::from_secs(300)), // 5 minutes
            retry_attempts: 1,
            retry_delay: Duration::from_millis(50),
            health_check_interval: None, // Skip health checks
        }
    }

    /// Recommend a configuration based on expected queries per second.
    ///
    /// # Arguments
    ///
    /// * `qps` - Expected queries per second
    /// * `avg_query_ms` - Average query duration in milliseconds
    ///
    /// # Example
    ///
    /// ```rust
    /// use prax_query::connection::PoolConfig;
    ///
    /// // 100 queries/sec with 10ms average latency
    /// let config = PoolConfig::for_workload(100, 10);
    /// assert!(config.pool.max_connections >= 5);
    /// ```
    pub fn for_workload(qps: u32, avg_query_ms: u32) -> Self {
        // Little's Law: connections = throughput * latency
        // Add 20% headroom
        let estimated_connections = ((qps * avg_query_ms) / 1000 + 1) * 120 / 100;
        let max_connections = estimated_connections.clamp(5, 100) as u32;
        let min_connections = (max_connections / 5).max(1);

        Self {
            connection: ConnectionOptions::new().connect_timeout(Duration::from_secs(10)),
            pool: PoolOptions::new()
                .max_connections(max_connections)
                .min_connections(min_connections)
                .acquire_timeout(Duration::from_secs(15))
                .idle_timeout(Duration::from_secs(300)),
            retry_attempts: 2,
            retry_delay: Duration::from_millis(200),
            health_check_interval: Some(Duration::from_secs(30)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig::new()
            .max_connections(30)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(10))
            .retry_attempts(5);

        assert_eq!(config.pool.max_connections, 30);
        assert_eq!(config.pool.min_connections, 5);
        assert_eq!(config.connection.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.retry_attempts, 5);
    }

    #[test]
    fn test_preset_configs() {
        let low_latency = PoolConfig::low_latency();
        assert_eq!(low_latency.pool.max_connections, 20);
        assert_eq!(low_latency.retry_attempts, 1);

        let high_throughput = PoolConfig::high_throughput();
        assert_eq!(high_throughput.pool.max_connections, 50);

        let dev = PoolConfig::development();
        assert_eq!(dev.pool.max_connections, 5);
        assert_eq!(dev.retry_attempts, 0);
    }
}
