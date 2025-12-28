import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-advanced-caching-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './advanced-caching.page.html',
})
export class AdvancedCachingPage {
  memoryCode = `use prax_query::data_cache::{CacheManager, MemoryCache};
use std::time::Duration;

let cache = CacheManager::new(
    MemoryCache::builder()
        .max_capacity(10_000)
        .time_to_live(Duration::from_secs(300))
        .build()
);

// Cache a user
let key = CacheKey::entity_record("User", user_id);
cache.set(&key, &user, None).await?;

// Retrieve from cache
let user: Option<User> = cache.get(&key).await?;`;

  tieredCode = `use prax_query::data_cache::{TieredCache, MemoryCache, RedisCache};

// L1: Fast in-memory cache
let memory = MemoryCache::builder()
    .max_capacity(1000)
    .time_to_live(Duration::from_secs(60))
    .build();

// L2: Distributed Redis cache
let redis = RedisCache::new(
    RedisCacheConfig::new("redis://localhost:6379")
).await?;

// Tiered: Check L1 first, fall back to L2
let cache = CacheManager::new(TieredCache::new(memory, redis));

// L1 hit: < 1ms
// L2 hit: 1-5ms
// Miss: fetch from database`;

  invalidateCode = `// Invalidate specific record
cache.invalidate_record("User", user_id).await?;

// Invalidate all User entries
cache.invalidate_pattern(&KeyPattern::entity("User")).await?;

// Tag-based invalidation
cache.invalidate_tags(&[
    EntityTag::tenant("tenant-123"),
    EntityTag::entity("Order"),
]).await?;

// Pattern matching
cache.invalidate_pattern(&KeyPattern::prefix("user:active:")).await?;`;

  keyCode = `use prax_query::data_cache::CacheKey;

// Entity record key
let key = CacheKey::entity_record("User", 123);
// Result: "User:id:123"

// With tenant context
let key = CacheKey::new("User", "id", &123)
    .with_tenant("tenant-456");
// Result: "tenant:tenant-456:User:id:123"

// Query result key
let key = CacheKey::query("users_by_status")
    .with_param("status", "active")
    .with_param("page", "1");`;

  metricsCode = `// Get cache statistics
let stats = cache.stats().await;

println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
println!("Total hits: {}", stats.hits);
println!("Total misses: {}", stats.misses);
println!("Evictions: {}", stats.evictions);
println!("Current size: {}", stats.size);`;
}

