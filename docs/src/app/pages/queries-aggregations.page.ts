import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-aggregations-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-aggregations.page.html',
})
export class QueriesAggregationsPage {
  countCode = `// Simple count
let total = client
    .user()
    .count()
    .exec()
    .await?;

// Filtered count
let active_users = client
    .user()
    .count()
    .where(user::active::equals(true))
    .exec()
    .await?;`;

  aggregateCode = `// Multiple aggregations
let stats = client
    .user()
    .aggregate()
    .count()
    .avg(user::age)
    .sum(user::points)
    .min(user::createdAt)
    .max(user::lastLogin)
    .exec()
    .await?;

println!("Total users: {}", stats.count);
println!("Average age: {}", stats.avg_age.unwrap_or(0.0));
println!("Total points: {}", stats.sum_points.unwrap_or(0));`;

  groupByCode = `// Group by with aggregations
let stats_by_role = client
    .user()
    .group_by(vec![user::role])
    .count()
    .avg(user::age)
    .exec()
    .await?;

for group in stats_by_role {
    println!(
        "Role {:?}: {} users, avg age {}",
        group.role,
        group.count,
        group.avg_age.unwrap_or(0.0)
    );
}

// Group by with having clause
let popular_roles = client
    .user()
    .group_by(vec![user::role])
    .count()
    .having(user::count::gte(10))
    .exec()
    .await?;`;
}
