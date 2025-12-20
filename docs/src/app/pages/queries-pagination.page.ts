import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-pagination-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-pagination.page.html',
})
export class QueriesPaginationPage {
  offsetPagination = `// Skip and take (offset pagination)
let page = 2;
let page_size = 20;

let users = client
    .user()
    .find_many()
    .skip((page - 1) * page_size)
    .take(page_size)
    .exec()
    .await?;

// Get total count for pagination UI
let total = client
    .user()
    .count()
    .exec()
    .await?;

let total_pages = (total + page_size - 1) / page_size;`;

  cursorPagination = `// Cursor-based pagination (more efficient for large datasets)
let users = client
    .user()
    .find_many()
    .cursor(user::id::equals(last_id))
    .take(20)
    .exec()
    .await?;

// Get the cursor for the next page
let next_cursor = users.last().map(|u| u.id);`;

  sorting = `// Single field sort
let users = client
    .user()
    .find_many()
    .order_by(user::createdAt::desc())
    .exec()
    .await?;

// Multiple field sort
let users = client
    .user()
    .find_many()
    .order_by(user::lastName::asc())
    .order_by(user::firstName::asc())
    .exec()
    .await?;

// Nulls first/last
let users = client
    .user()
    .find_many()
    .order_by(user::deletedAt::desc().nulls_first())
    .exec()
    .await?;`;

  distinct = `// Get distinct values
let cities = client
    .user()
    .find_many()
    .distinct(vec![user::city])
    .exec()
    .await?;

// Distinct on multiple fields
let locations = client
    .user()
    .find_many()
    .distinct(vec![user::country, user::city])
    .exec()
    .await?;`;
}
