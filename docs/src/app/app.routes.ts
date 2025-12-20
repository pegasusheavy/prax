import { Routes } from '@angular/router';

export const routes: Routes = [
  {
    path: '',
    loadComponent: () => import('./pages/home.page').then(m => m.HomePage),
  },
  {
    path: 'quickstart',
    loadComponent: () => import('./pages/quickstart.page').then(m => m.QuickstartPage),
  },
  {
    path: 'installation',
    loadComponent: () => import('./pages/installation.page').then(m => m.InstallationPage),
  },
  {
    path: 'configuration',
    loadComponent: () => import('./pages/config-reference.page').then(m => m.ConfigReferencePage),
  },
  {
    path: 'performance',
    loadComponent: () => import('./pages/performance.page').then(m => m.PerformancePage),
  },
  {
    path: 'schema/overview',
    loadComponent: () => import('./pages/schema-overview.page').then(m => m.SchemaOverviewPage),
  },
  {
    path: 'schema/models',
    loadComponent: () => import('./pages/schema-models.page').then(m => m.SchemaModelsPage),
  },
  {
    path: 'schema/fields',
    loadComponent: () => import('./pages/schema-fields.page').then(m => m.SchemaFieldsPage),
  },
  {
    path: 'schema/relations',
    loadComponent: () => import('./pages/schema-relations.page').then(m => m.SchemaRelationsPage),
  },
  {
    path: 'schema/attributes',
    loadComponent: () => import('./pages/schema-attributes.page').then(m => m.SchemaAttributesPage),
  },
  {
    path: 'schema/enums',
    loadComponent: () => import('./pages/schema-enums.page').then(m => m.SchemaEnumsPage),
  },
  {
    path: 'schema/views',
    loadComponent: () => import('./pages/schema-views.page').then(m => m.SchemaViewsPage),
  },
  {
    path: 'schema/generators',
    loadComponent: () => import('./pages/schema-generators.page').then(m => m.SchemaGeneratorsPage),
  },
  {
    path: 'schema/server-groups',
    loadComponent: () => import('./pages/schema-server-groups.page').then(m => m.SchemaServerGroupsPage),
  },
  {
    path: 'queries/crud',
    loadComponent: () => import('./pages/queries-crud.page').then(m => m.QueriesCrudPage),
  },
  {
    path: 'queries/filtering',
    loadComponent: () => import('./pages/queries-filtering.page').then(m => m.QueriesFilteringPage),
  },
  {
    path: 'queries/pagination',
    loadComponent: () => import('./pages/queries-pagination.page').then(m => m.QueriesPaginationPage),
  },
  {
    path: 'queries/aggregations',
    loadComponent: () => import('./pages/queries-aggregations.page').then(m => m.QueriesAggregationsPage),
  },
  {
    path: 'queries/raw-sql',
    loadComponent: () => import('./pages/queries-raw-sql.page').then(m => m.QueriesRawSqlPage),
  },
  {
    path: 'database/postgresql',
    loadComponent: () => import('./pages/database-postgresql.page').then(m => m.DatabasePostgresqlPage),
  },
  {
    path: 'database/mysql',
    loadComponent: () => import('./pages/database-mysql.page').then(m => m.DatabaseMysqlPage),
  },
  {
    path: 'database/sqlite',
    loadComponent: () => import('./pages/database-sqlite.page').then(m => m.DatabaseSqlitePage),
  },
  {
    path: 'database/migrations',
    loadComponent: () => import('./pages/database-migrations.page').then(m => m.DatabaseMigrationsPage),
  },
  {
    path: 'integrations/armature',
    loadComponent: () => import('./pages/integration-armature.page').then(m => m.IntegrationArmaturePage),
  },
  {
    path: 'integrations/axum',
    loadComponent: () => import('./pages/integration-axum.page').then(m => m.IntegrationAxumPage),
  },
  {
    path: 'integrations/actix',
    loadComponent: () => import('./pages/integration-actix.page').then(m => m.IntegrationActixPage),
  },
  {
    path: 'examples',
    loadComponent: () => import('./pages/examples.page').then(m => m.ExamplesPage),
  },
  {
    path: 'advanced/connection',
    loadComponent: () => import('./pages/advanced-connection.page').then(m => m.AdvancedConnectionPage),
  },
  {
    path: 'advanced/middleware',
    loadComponent: () => import('./pages/advanced-middleware.page').then(m => m.AdvancedMiddlewarePage),
  },
  {
    path: 'advanced/errors',
    loadComponent: () => import('./pages/advanced-errors.page').then(m => m.AdvancedErrorsPage),
  },
  {
    path: '**',
    redirectTo: '',
  },
];
