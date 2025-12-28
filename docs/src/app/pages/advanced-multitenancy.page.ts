import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-advanced-multitenancy-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './advanced-multitenancy.page.html',
})
export class AdvancedMultitenancyPage {
  taskLocalCode = `use prax_query::tenant::task_local::with_tenant;

// Zero-allocation tenant context
with_tenant("tenant-123", async {
    // All code here sees tenant-123
    let users = client.user().find_many().exec().await?;
    Ok(())
}).await?;`;

  rlsCode = `use prax_query::tenant::rls::{RlsManager, RlsConfig};

let rls = RlsManager::new(
    RlsConfig::new("tenant_id")
        .with_session_variable("app.current_tenant")
        .add_tables(["users", "orders", "products"])
);

// One-time setup (creates RLS policies)
conn.execute_batch(&rls.setup_sql()).await?;

// Per-request: just set the tenant
conn.execute(&rls.set_tenant_local_sql("tenant-123"), &[]).await?;

// All queries now automatically filtered by tenant_id`;

  cacheCode = `use prax_query::tenant::cache::ShardedTenantCache;

// High-concurrency sharded cache
let cache = ShardedTenantCache::high_concurrency(10_000);

// Get or fetch tenant context
let ctx = cache.get_or_fetch(&tenant_id, || async {
    db_lookup_tenant(&tenant_id).await
}).await?;`;

  poolCode = `use prax_query::tenant::pool::TenantPoolManager;

// Per-tenant connection pools
let manager = TenantPoolManager::new(db_config);

// Get connection for specific tenant
let client = manager.get_client(&tenant_id).await?;

// Execute queries with isolated connection
let results = client.query("SELECT * FROM data", &[]).await?;`;

  preparedCode = `use prax_query::tenant::prepared::PreparedStatementCache;

// Global statement cache
let cache = PreparedStatementCache::global();
let stmt = cache.get_or_prepare("SELECT * FROM users WHERE id = $1").await?;

// Per-tenant statement cache
let tenant_cache = cache.for_tenant(&tenant_id);
let stmt = tenant_cache.get_or_prepare("SELECT * FROM orders").await?;`;
}

