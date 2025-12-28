import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-search-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-search.page.html',
})
export class QueriesSearchPage {
  basicSearch = `use prax::search::{SearchQuery, SearchMode};

// Basic full-text search
let results = SearchQuery::new("wireless bluetooth headphones")
    .columns(["name", "description"])
    .mode(SearchMode::Natural)  // Natural language search
    .build_postgres();

// PostgreSQL: SELECT * FROM products
//   WHERE to_tsvector('english', name || ' ' || description)
//         @@ plainto_tsquery('english', $1)

// MySQL: SELECT * FROM products
//   WHERE MATCH(name, description) AGAINST($1 IN NATURAL LANGUAGE MODE)

// SQLite FTS5: SELECT * FROM products_fts
//   WHERE products_fts MATCH $1`;

  searchWithRanking = `use prax::search::{SearchQuery, RankingOptions};

// Search with relevance ranking
let results = SearchQuery::new("rust async database")
    .columns(["title", "content", "tags"])
    .ranking(RankingOptions::new()
        .weight("title", 'A')      // Highest priority
        .weight("tags", 'B')       // Medium priority
        .weight("content", 'C')    // Lower priority
        .normalization(32)         // Document length normalization
    )
    .order_by_rank()
    .limit(20)
    .build_postgres();

// PostgreSQL:
// SELECT *, ts_rank_cd(
//   setweight(to_tsvector('english', title), 'A') ||
//   setweight(to_tsvector('english', tags), 'B') ||
//   setweight(to_tsvector('english', content), 'C'),
//   plainto_tsquery('english', $1), 32
// ) AS rank
// FROM articles
// ORDER BY rank DESC
// LIMIT 20`;

  searchWithHighlight = `use prax::search::{SearchQuery, HighlightOptions};

// Search with highlighted snippets
let results = SearchQuery::new("machine learning")
    .columns(["title", "content"])
    .highlight(HighlightOptions::new()
        .start_tag("<mark>")
        .end_tag("</mark>")
        .max_words(35)
        .min_words(15)
        .short_word(3)
        .max_fragments(3)
    )
    .build_postgres();

// Returns:
// {
//   id: 1,
//   title: "Introduction to <mark>Machine</mark> <mark>Learning</mark>",
//   content_highlight: "...deep <mark>learning</mark> is a subset of <mark>machine</mark>..."
// }

// PostgreSQL: ts_headline('english', content, query, 'StartSel=<mark>, StopSel=</mark>')
// MySQL: No native highlight, use application-side
// MSSQL: Use CONTAINS with ISABOUT for similar ranking`;

  fuzzySearch = `use prax::search::{SearchQuery, FuzzyOptions};

// Fuzzy search (typo-tolerant)
let results = SearchQuery::new("wireles headfones")  // Typos!
    .columns(["name"])
    .fuzzy(FuzzyOptions::new()
        .max_edits(2)           // Levenshtein distance
        .prefix_length(2)       // Don't fuzzy first N chars
        .transpositions(true)   // Allow ab->ba
    )
    .build_postgres();

// PostgreSQL with pg_trgm:
// SELECT *, similarity(name, $1) AS sim
// FROM products
// WHERE name % $1  -- Trigram similarity operator
// ORDER BY sim DESC

// MSSQL with SOUNDEX:
// SELECT * FROM products
// WHERE SOUNDEX(name) = SOUNDEX(@p1)`;

  phraseSearch = `use prax::search::{SearchQuery, SearchMode};

// Exact phrase search
let results = SearchQuery::new("rust programming language")
    .columns(["content"])
    .mode(SearchMode::Phrase)
    .build_postgres();

// PostgreSQL: phraseto_tsquery('english', $1)
//   Matches "rust programming language" as consecutive words

// Boolean search with operators
let results = SearchQuery::new("rust & (async | tokio) & !python")
    .columns(["content"])
    .mode(SearchMode::Boolean)
    .build_postgres();

// PostgreSQL: to_tsquery('english', 'rust & (async | tokio) & !python')`;

  searchIndex = `use prax::search::{FullTextIndex, FullTextIndexBuilder};

// Create full-text search index
let index = FullTextIndexBuilder::new("products_search_idx")
    .table("products")
    .columns(["name", "description", "tags"])
    .language("english")
    .build();

// PostgreSQL GIN index:
// CREATE INDEX products_search_idx ON products
//   USING GIN (to_tsvector('english', name || ' ' || description || ' ' || tags))

// MySQL FULLTEXT index:
// ALTER TABLE products ADD FULLTEXT INDEX products_search_idx (name, description, tags)

// SQLite FTS5 virtual table:
// CREATE VIRTUAL TABLE products_fts USING fts5(name, description, tags, content='products')

// MSSQL Full-Text Catalog:
// CREATE FULLTEXT INDEX ON products (name, description, tags)
//   KEY INDEX PK_products ON products_catalog`;

  atlasSearch = `use prax::search::mongodb::{AtlasSearchQuery, AtlasSearchIndexBuilder};

// Create Atlas Search index
let index = AtlasSearchIndexBuilder::new("product_search")
    .collection("products")
    .dynamic_mapping(false)
    .field("name", "string", [
        ("analyzer", "lucene.standard"),
        ("searchAnalyzer", "lucene.standard"),
    ])
    .field("description", "string", [
        ("analyzer", "lucene.english"),
    ])
    .field("price", "number")
    .field("category", "stringFacet")
    .field("location", "geo")
    .build();

// Full-text search with Atlas Search
let results = AtlasSearchQuery::new("wireless headphones")
    .index("product_search")
    .path(["name", "description"])
    .fuzzy(2, 3)  // maxEdits, prefixLength
    .highlight(["name", "description"])
    .score_boost("name", 3.0)  // Boost name matches
    .compound()
        .must(text_query)
        .filter(range("price").lt(200))
        .should(near("location", geo_point, 10))
    .facets([
        string_facet("category", 10),
        numeric_facet("price", [0, 50, 100, 200, 500]),
    ])
    .limit(20)
    .exec(&client)
    .await?;

// Access results
for hit in results.hits {
    println!("Score: {:.2}", hit.score);
    println!("Name: {}", hit.document.name);
    for h in hit.highlights {
        println!("  {} in {}", h.texts.join("..."), h.path);
    }
}

// Access facets
for (category, count) in results.facets["category"].buckets {
    println!("{}: {} products", category, count);
}`;

  searchMigration = `// Schema with full-text search
model Article {
  id        Int      @id @auto
  title     String
  content   String   @db.Text
  tags      String[]

  // Full-text search index
  @@fulltext([title, content, tags], name: "article_search")
}

// Migration for search index
-- PostgreSQL
CREATE INDEX article_search ON articles
  USING GIN (to_tsvector('english', title || ' ' || content || ' ' || array_to_string(tags, ' ')));

-- MySQL
ALTER TABLE articles ADD FULLTEXT INDEX article_search (title, content);

-- SQLite (requires FTS5 virtual table)
CREATE VIRTUAL TABLE articles_fts USING fts5(
  title, content, tags,
  content='articles',
  content_rowid='id'
);

CREATE TRIGGER articles_ai AFTER INSERT ON articles BEGIN
  INSERT INTO articles_fts(rowid, title, content, tags)
  VALUES (new.id, new.title, new.content, new.tags);
END;`;
}
