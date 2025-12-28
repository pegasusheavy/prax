//! Row-Level Security (RLS) support for Microsoft SQL Server.
//!
//! SQL Server implements RLS through Security Policies with predicate functions.
//! This module provides tools to generate these security policies from Prax schemas.
//!
//! ## SQL Server RLS Architecture
//!
//! 1. **Predicate Function**: A schema-bound inline table-valued function that
//!    returns 1 for rows that should be accessible.
//!
//! 2. **Security Policy**: Binds predicate functions to tables with filter and
//!    block predicates.
//!
//! ## Example Generated SQL
//!
//! ```sql
//! -- Create schema for security objects
//! CREATE SCHEMA Security;
//! GO
//!
//! -- Create predicate function
//! CREATE FUNCTION Security.fn_user_filter(@UserId INT)
//!     RETURNS TABLE
//! WITH SCHEMABINDING
//! AS
//!     RETURN SELECT 1 AS result
//!     WHERE @UserId = CAST(SESSION_CONTEXT(N'UserId') AS INT);
//! GO
//!
//! -- Create security policy
//! CREATE SECURITY POLICY Security.UserPolicy
//! ADD FILTER PREDICATE Security.fn_user_filter(UserId) ON dbo.Users,
//! ADD BLOCK PREDICATE Security.fn_user_filter(UserId) ON dbo.Users AFTER INSERT
//! WITH (STATE = ON);
//! ```

use prax_schema::{Policy, PolicyCommand};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::error::MssqlResult;

/// MSSQL block operation types for security policies.
///
/// Block predicates control when data modification is allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockOperation {
    /// Block predicate evaluated after INSERT.
    /// Prevents inserting rows that don't satisfy the predicate.
    AfterInsert,
    /// Block predicate evaluated after UPDATE.
    /// Prevents updating rows to values that don't satisfy the predicate.
    AfterUpdate,
    /// Block predicate evaluated before UPDATE.
    /// Prevents updating rows that currently don't satisfy the predicate.
    BeforeUpdate,
    /// Block predicate evaluated before DELETE.
    /// Prevents deleting rows that don't satisfy the predicate.
    BeforeDelete,
}

impl BlockOperation {
    /// Parse a block operation from a string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().replace([' ', '_'], "").as_str() {
            "AFTERINSERT" => Some(Self::AfterInsert),
            "AFTERUPDATE" => Some(Self::AfterUpdate),
            "BEFOREUPDATE" => Some(Self::BeforeUpdate),
            "BEFOREDELETE" => Some(Self::BeforeDelete),
            _ => None,
        }
    }

    /// Get the SQL clause for this block operation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AfterInsert => "AFTER INSERT",
            Self::AfterUpdate => "AFTER UPDATE",
            Self::BeforeUpdate => "BEFORE UPDATE",
            Self::BeforeDelete => "BEFORE DELETE",
        }
    }
}

impl std::fmt::Display for BlockOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Generated SQL Server security policy.
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Schema name for security objects.
    pub schema: SmolStr,
    /// Policy name.
    pub name: SmolStr,
    /// Target table (fully qualified, e.g., "dbo.Users").
    pub table: String,
    /// Predicate column name.
    pub predicate_column: String,
    /// Filter predicate expression.
    pub filter_expression: Option<String>,
    /// Block predicates with their operations.
    pub block_predicates: Vec<(BlockOperation, String)>,
    /// Whether the policy should be enabled.
    pub enabled: bool,
}

impl SecurityPolicy {
    /// Create a new security policy.
    pub fn new(
        schema: impl Into<SmolStr>,
        name: impl Into<SmolStr>,
        table: impl Into<String>,
        predicate_column: impl Into<String>,
    ) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
            table: table.into(),
            predicate_column: predicate_column.into(),
            filter_expression: None,
            block_predicates: Vec::new(),
            enabled: true,
        }
    }

    /// Set the filter expression (for SELECT visibility).
    pub fn with_filter(mut self, expression: impl Into<String>) -> Self {
        self.filter_expression = Some(expression.into());
        self
    }

    /// Add a block predicate.
    pub fn with_block(mut self, operation: BlockOperation, expression: impl Into<String>) -> Self {
        self.block_predicates.push((operation, expression.into()));
        self
    }

    /// Set whether the policy is enabled.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Get the predicate function name.
    pub fn function_name(&self) -> String {
        format!("fn_{}_predicate", self.name)
    }

    /// Generate the CREATE SCHEMA statement.
    pub fn schema_sql(&self) -> String {
        format!(
            r#"IF NOT EXISTS (SELECT * FROM sys.schemas WHERE name = N'{schema}')
BEGIN
    EXEC('CREATE SCHEMA [{schema}]')
END"#,
            schema = self.schema
        )
    }

    /// Generate the CREATE FUNCTION statement for the predicate.
    pub fn function_sql(&self) -> String {
        let func_name = self.function_name();
        let expression = self.filter_expression.as_deref().unwrap_or("1 = 1");

        format!(
            r#"CREATE FUNCTION [{schema}].[{func_name}](@{col} INT)
    RETURNS TABLE
WITH SCHEMABINDING
AS
    RETURN SELECT 1 AS fn_securitypredicate_result
    WHERE {expression}"#,
            schema = self.schema,
            func_name = func_name,
            col = self.predicate_column,
            expression = expression
        )
    }

    /// Generate the CREATE SECURITY POLICY statement.
    pub fn policy_sql(&self) -> String {
        let func_name = self.function_name();
        let mut parts = Vec::new();

        // Add filter predicate if we have a filter expression
        if self.filter_expression.is_some() {
            parts.push(format!(
                "ADD FILTER PREDICATE [{schema}].[{func_name}]({col}) ON {table}",
                schema = self.schema,
                func_name = func_name,
                col = self.predicate_column,
                table = self.table
            ));
        }

        // Add block predicates
        for (op, _expr) in &self.block_predicates {
            parts.push(format!(
                "ADD BLOCK PREDICATE [{schema}].[{func_name}]({col}) ON {table} {op}",
                schema = self.schema,
                func_name = func_name,
                col = self.predicate_column,
                table = self.table,
                op = op.as_str()
            ));
        }

        let predicates = parts.join(",\n");
        let state = if self.enabled { "ON" } else { "OFF" };

        format!(
            r#"CREATE SECURITY POLICY [{schema}].[{name}]
{predicates}
WITH (STATE = {state})"#,
            schema = self.schema,
            name = self.name,
            predicates = predicates,
            state = state
        )
    }

    /// Generate the DROP statements for cleanup.
    pub fn drop_sql(&self) -> String {
        let func_name = self.function_name();
        format!(
            r#"IF EXISTS (SELECT * FROM sys.security_policies WHERE name = N'{name}' AND schema_id = SCHEMA_ID(N'{schema}'))
    DROP SECURITY POLICY [{schema}].[{name}];

IF EXISTS (SELECT * FROM sys.objects WHERE object_id = OBJECT_ID(N'[{schema}].[{func_name}]') AND type = 'IF')
    DROP FUNCTION [{schema}].[{func_name}];"#,
            schema = self.schema,
            name = self.name,
            func_name = func_name
        )
    }

    /// Generate all SQL statements for the complete policy setup.
    pub fn to_sql(&self) -> String {
        format!(
            "{schema_sql};\nGO\n\n{function_sql};\nGO\n\n{policy_sql};",
            schema_sql = self.schema_sql(),
            function_sql = self.function_sql(),
            policy_sql = self.policy_sql()
        )
    }

    /// Generate migration-safe SQL that drops existing policy first.
    pub fn to_migration_sql(&self) -> String {
        format!(
            "{drop_sql}\nGO\n\n{create_sql}",
            drop_sql = self.drop_sql(),
            create_sql = self.to_sql()
        )
    }
}

/// Generator for SQL Server security policies from Prax schema policies.
pub struct SecurityPolicyGenerator {
    /// Default schema for security objects.
    default_schema: SmolStr,
    /// Whether to use SESSION_CONTEXT for user identity.
    use_session_context: bool,
    /// Session context key for user ID.
    user_id_key: SmolStr,
    /// Session context key for tenant/org ID.
    tenant_id_key: SmolStr,
}

impl Default for SecurityPolicyGenerator {
    fn default() -> Self {
        Self::new("Security")
    }
}

impl SecurityPolicyGenerator {
    /// Create a new generator with the given default schema.
    pub fn new(default_schema: impl Into<SmolStr>) -> Self {
        Self {
            default_schema: default_schema.into(),
            use_session_context: true,
            user_id_key: "UserId".into(),
            tenant_id_key: "TenantId".into(),
        }
    }

    /// Set whether to use SESSION_CONTEXT for user identity.
    pub fn with_session_context(mut self, use_it: bool) -> Self {
        self.use_session_context = use_it;
        self
    }

    /// Set the session context key for user ID.
    pub fn with_user_id_key(mut self, key: impl Into<SmolStr>) -> Self {
        self.user_id_key = key.into();
        self
    }

    /// Set the session context key for tenant/org ID.
    pub fn with_tenant_id_key(mut self, key: impl Into<SmolStr>) -> Self {
        self.tenant_id_key = key.into();
        self
    }

    /// Generate a security policy from a Prax policy.
    pub fn generate(
        &self,
        policy: &Policy,
        table: &str,
        predicate_column: &str,
    ) -> MssqlResult<SecurityPolicy> {
        // Determine the schema to use
        let schema = policy
            .mssql_schema
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or(self.default_schema.as_str());

        let mut security_policy =
            SecurityPolicy::new(schema, policy.name(), table, predicate_column);

        // Convert the USING expression to a filter predicate
        if let Some(ref using_expr) = policy.using_expr {
            let converted = self.convert_expression(using_expr, predicate_column);
            security_policy = security_policy.with_filter(converted);
        }

        // Convert the CHECK expression to block predicates
        if let Some(ref check_expr) = policy.check_expr {
            let converted = self.convert_expression(check_expr, predicate_column);
            let block_ops = self.determine_block_operations(policy);

            for op in block_ops {
                security_policy = security_policy.with_block(op, converted.clone());
            }
        }

        Ok(security_policy)
    }

    /// Convert PostgreSQL-style expressions to MSSQL.
    fn convert_expression(&self, expr: &str, predicate_column: &str) -> String {
        let mut result = expr.to_string();

        // Convert PostgreSQL functions to MSSQL equivalents
        result = result.replace(
            "current_user_id()",
            &format!("CAST(SESSION_CONTEXT(N'{}') AS INT)", self.user_id_key),
        );

        result = result.replace(
            "auth.uid()",
            &format!("CAST(SESSION_CONTEXT(N'{}') AS INT)", self.user_id_key),
        );

        result = result.replace(
            "current_setting('app.current_org')",
            &format!("SESSION_CONTEXT(N'{}')", self.tenant_id_key),
        );

        result = result.replace(
            "current_setting('app.current_tenant')",
            &format!("SESSION_CONTEXT(N'{}')", self.tenant_id_key),
        );

        // Convert PostgreSQL :: cast to MSSQL CAST
        // e.g., ::uuid becomes CAST(... AS UNIQUEIDENTIFIER)
        // This is a simplified conversion - complex casts may need manual handling
        if result.contains("::int") {
            result = result.replace("::int", "");
        }
        if result.contains("::uuid") {
            result = result.replace("::uuid", "");
        }

        // Replace column references with parameter reference
        // This assumes the expression uses the column name directly
        if !result.contains(&format!("@{}", predicate_column)) {
            // If the expression references the column directly, use the parameter
            result = result.replace(predicate_column, &format!("@{}", predicate_column));
        }

        result
    }

    /// Determine which block operations to use based on the policy commands.
    fn determine_block_operations(&self, policy: &Policy) -> Vec<BlockOperation> {
        // If the policy has explicit MSSQL block operations, use those
        if !policy.mssql_block_operations.is_empty() {
            return policy
                .mssql_block_operations
                .iter()
                .map(|op| match op {
                    prax_schema::MssqlBlockOperation::AfterInsert => BlockOperation::AfterInsert,
                    prax_schema::MssqlBlockOperation::AfterUpdate => BlockOperation::AfterUpdate,
                    prax_schema::MssqlBlockOperation::BeforeUpdate => BlockOperation::BeforeUpdate,
                    prax_schema::MssqlBlockOperation::BeforeDelete => BlockOperation::BeforeDelete,
                })
                .collect();
        }

        // Otherwise, infer from the policy commands
        let mut ops = Vec::new();

        if policy.applies_to(PolicyCommand::Insert) {
            ops.push(BlockOperation::AfterInsert);
        }
        if policy.applies_to(PolicyCommand::Update) {
            ops.push(BlockOperation::BeforeUpdate);
            ops.push(BlockOperation::AfterUpdate);
        }
        if policy.applies_to(PolicyCommand::Delete) {
            ops.push(BlockOperation::BeforeDelete);
        }

        ops
    }

    /// Generate security policies for all policies in a schema.
    pub fn generate_all(
        &self,
        policies: &[&Policy],
        table_mapping: &impl Fn(&str) -> (String, String), // model name -> (table, column)
    ) -> MssqlResult<Vec<SecurityPolicy>> {
        policies
            .iter()
            .map(|policy| {
                let (table, column) = table_mapping(policy.table());
                self.generate(policy, &table, &column)
            })
            .collect()
    }
}

/// Builder for setting up RLS on a connection.
pub struct RlsContextBuilder {
    user_id: Option<String>,
    tenant_id: Option<String>,
    custom_values: Vec<(String, String)>,
    read_only: bool,
}

impl Default for RlsContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RlsContextBuilder {
    /// Create a new RLS context builder.
    pub fn new() -> Self {
        Self {
            user_id: None,
            tenant_id: None,
            custom_values: Vec::new(),
            read_only: true,
        }
    }

    /// Set the user ID for the session.
    pub fn user_id(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }

    /// Set the tenant/org ID for the session.
    pub fn tenant_id(mut self, id: impl Into<String>) -> Self {
        self.tenant_id = Some(id.into());
        self
    }

    /// Add a custom session context value.
    pub fn custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_values.push((key.into(), value.into()));
        self
    }

    /// Set whether the context values should be read-only.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Generate the SQL to set up the session context.
    pub fn to_sql(&self) -> String {
        let read_only = if self.read_only { 1 } else { 0 };
        let mut statements = Vec::new();

        if let Some(ref user_id) = self.user_id {
            statements.push(format!(
                "EXEC sp_set_session_context @key = N'UserId', @value = N'{}', @read_only = {}",
                user_id.replace('\'', "''"),
                read_only
            ));
        }

        if let Some(ref tenant_id) = self.tenant_id {
            statements.push(format!(
                "EXEC sp_set_session_context @key = N'TenantId', @value = N'{}', @read_only = {}",
                tenant_id.replace('\'', "''"),
                read_only
            ));
        }

        for (key, value) in &self.custom_values {
            statements.push(format!(
                "EXEC sp_set_session_context @key = N'{}', @value = N'{}', @read_only = {}",
                key.replace('\'', "''"),
                value.replace('\'', "''"),
                read_only
            ));
        }

        statements.join(";\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prax_schema::{Ident, Span};

    fn make_span() -> Span {
        Span::new(0, 10)
    }

    fn make_ident(name: &str) -> Ident {
        Ident::new(name, make_span())
    }

    fn make_policy(name: &str, table: &str) -> Policy {
        Policy::new(make_ident(name), make_ident(table), make_span())
    }

    // ==================== BlockOperation Tests ====================

    #[test]
    fn test_block_operation_from_str() {
        assert_eq!(
            BlockOperation::from_str("AFTER INSERT"),
            Some(BlockOperation::AfterInsert)
        );
        assert_eq!(
            BlockOperation::from_str("after_insert"),
            Some(BlockOperation::AfterInsert)
        );
        assert_eq!(
            BlockOperation::from_str("BEFOREDELETE"),
            Some(BlockOperation::BeforeDelete)
        );
        assert_eq!(BlockOperation::from_str("invalid"), None);
    }

    #[test]
    fn test_block_operation_as_str() {
        assert_eq!(BlockOperation::AfterInsert.as_str(), "AFTER INSERT");
        assert_eq!(BlockOperation::BeforeDelete.as_str(), "BEFORE DELETE");
    }

    // ==================== SecurityPolicy Tests ====================

    #[test]
    fn test_security_policy_new() {
        let policy = SecurityPolicy::new("Security", "UserFilter", "dbo.Users", "UserId");

        assert_eq!(policy.schema.as_str(), "Security");
        assert_eq!(policy.name.as_str(), "UserFilter");
        assert_eq!(policy.table, "dbo.Users");
        assert_eq!(policy.predicate_column, "UserId");
        assert!(policy.enabled);
    }

    #[test]
    fn test_security_policy_function_name() {
        let policy = SecurityPolicy::new("Security", "UserFilter", "dbo.Users", "UserId");
        assert_eq!(policy.function_name(), "fn_UserFilter_predicate");
    }

    #[test]
    fn test_security_policy_schema_sql() {
        let policy = SecurityPolicy::new("Security", "Test", "dbo.Users", "UserId");
        let sql = policy.schema_sql();

        assert!(sql.contains("CREATE SCHEMA [Security]"));
        assert!(sql.contains("IF NOT EXISTS"));
    }

    #[test]
    fn test_security_policy_function_sql() {
        let policy = SecurityPolicy::new("Security", "Test", "dbo.Users", "UserId")
            .with_filter("@UserId = CAST(SESSION_CONTEXT(N'UserId') AS INT)");

        let sql = policy.function_sql();

        assert!(sql.contains("CREATE FUNCTION [Security].[fn_Test_predicate]"));
        assert!(sql.contains("@UserId INT"));
        assert!(sql.contains("RETURNS TABLE"));
        assert!(sql.contains("WITH SCHEMABINDING"));
        assert!(sql.contains("SESSION_CONTEXT"));
    }

    #[test]
    fn test_security_policy_policy_sql() {
        let policy = SecurityPolicy::new("Security", "Test", "dbo.Users", "UserId")
            .with_filter("@UserId = 1")
            .with_block(BlockOperation::AfterInsert, "@UserId = 1");

        let sql = policy.policy_sql();

        assert!(sql.contains("CREATE SECURITY POLICY [Security].[Test]"));
        assert!(sql.contains("ADD FILTER PREDICATE"));
        assert!(sql.contains("ADD BLOCK PREDICATE"));
        assert!(sql.contains("AFTER INSERT"));
        assert!(sql.contains("WITH (STATE = ON)"));
    }

    #[test]
    fn test_security_policy_drop_sql() {
        let policy = SecurityPolicy::new("Security", "Test", "dbo.Users", "UserId");
        let sql = policy.drop_sql();

        assert!(sql.contains("DROP SECURITY POLICY"));
        assert!(sql.contains("DROP FUNCTION"));
        assert!(sql.contains("IF EXISTS"));
    }

    #[test]
    fn test_security_policy_to_sql() {
        let policy = SecurityPolicy::new("Security", "Test", "dbo.Users", "UserId")
            .with_filter("@UserId = 1");

        let sql = policy.to_sql();

        assert!(sql.contains("CREATE SCHEMA"));
        assert!(sql.contains("CREATE FUNCTION"));
        assert!(sql.contains("CREATE SECURITY POLICY"));
        assert!(sql.contains("GO"));
    }

    // ==================== SecurityPolicyGenerator Tests ====================

    #[test]
    fn test_generator_default() {
        let generator = SecurityPolicyGenerator::default();
        assert_eq!(generator.default_schema.as_str(), "Security");
    }

    #[test]
    fn test_generator_generate_simple() {
        let generator = SecurityPolicyGenerator::new("RLS");
        let policy = make_policy("UserFilter", "User").with_using("id = current_user_id()");

        let security = generator.generate(&policy, "dbo.Users", "UserId").unwrap();

        assert_eq!(security.schema.as_str(), "RLS");
        assert_eq!(security.name.as_str(), "UserFilter");
        assert!(security.filter_expression.is_some());
        assert!(
            security
                .filter_expression
                .as_ref()
                .unwrap()
                .contains("SESSION_CONTEXT")
        );
    }

    #[test]
    fn test_generator_convert_expression() {
        let generator = SecurityPolicyGenerator::new("Security");

        // Test current_user_id() conversion - the generator uses the user_id_key setting
        let expr = "id = current_user_id()";
        let converted = generator.convert_expression(expr, "UserId");
        assert!(
            converted.contains("SESSION_CONTEXT"),
            "Expected SESSION_CONTEXT in: {}",
            converted
        );
        assert!(
            converted.contains("UserId"),
            "Expected UserId in: {}",
            converted
        );

        // Test current_setting conversion - uses tenant_id_key setting
        let expr = "org_id = current_setting('app.current_org')";
        let converted = generator.convert_expression(expr, "OrgId");
        assert!(
            converted.contains("SESSION_CONTEXT"),
            "Expected SESSION_CONTEXT in: {}",
            converted
        );
        assert!(
            converted.contains("TenantId"),
            "Expected TenantId in: {}",
            converted
        );
    }

    #[test]
    fn test_generator_determine_block_operations() {
        let generator = SecurityPolicyGenerator::new("Security");

        // Test INSERT command
        let policy = make_policy("Test", "User").with_commands(vec![PolicyCommand::Insert]);
        let ops = generator.determine_block_operations(&policy);
        assert!(ops.contains(&BlockOperation::AfterInsert));
        assert!(!ops.contains(&BlockOperation::BeforeDelete));

        // Test ALL command
        let policy = make_policy("Test", "User").with_commands(vec![PolicyCommand::All]);
        let ops = generator.determine_block_operations(&policy);
        assert!(ops.contains(&BlockOperation::AfterInsert));
        assert!(ops.contains(&BlockOperation::BeforeUpdate));
        assert!(ops.contains(&BlockOperation::AfterUpdate));
        assert!(ops.contains(&BlockOperation::BeforeDelete));
    }

    // ==================== RlsContextBuilder Tests ====================

    #[test]
    fn test_rls_context_builder() {
        let sql = RlsContextBuilder::new()
            .user_id("123")
            .tenant_id("456")
            .to_sql();

        assert!(sql.contains("sp_set_session_context"));
        assert!(sql.contains("UserId"));
        assert!(sql.contains("TenantId"));
        assert!(sql.contains("123"));
        assert!(sql.contains("456"));
    }

    #[test]
    fn test_rls_context_builder_custom() {
        let sql = RlsContextBuilder::new()
            .custom("CustomKey", "CustomValue")
            .read_only(false)
            .to_sql();

        assert!(sql.contains("CustomKey"));
        assert!(sql.contains("CustomValue"));
        assert!(sql.contains("@read_only = 0"));
    }

    // ==================== Integration Scenario Tests ====================

    #[test]
    fn test_full_rls_scenario_user_isolation() {
        let generator = SecurityPolicyGenerator::new("Security");

        let policy = make_policy("UserIsolation", "User")
            .with_commands(vec![PolicyCommand::All])
            .with_using("id = current_user_id()")
            .with_check("id = current_user_id()");

        let security = generator.generate(&policy, "dbo.Users", "UserId").unwrap();
        let sql = security.to_sql();

        // Verify all required components are present
        assert!(sql.contains("CREATE SCHEMA"));
        assert!(sql.contains("CREATE FUNCTION"));
        assert!(sql.contains("WITH SCHEMABINDING"));
        assert!(sql.contains("CREATE SECURITY POLICY"));
        assert!(sql.contains("FILTER PREDICATE"));
        assert!(sql.contains("BLOCK PREDICATE"));
        assert!(sql.contains("SESSION_CONTEXT"));
    }

    #[test]
    fn test_full_rls_scenario_multi_tenant() {
        let generator =
            SecurityPolicyGenerator::new("MultiTenant").with_tenant_id_key("OrganizationId");

        let policy = make_policy("TenantIsolation", "Order")
            .with_using("org_id = current_setting('app.current_org')");

        let security = generator.generate(&policy, "dbo.Orders", "OrgId").unwrap();

        assert_eq!(security.schema.as_str(), "MultiTenant");
        assert!(
            security
                .filter_expression
                .as_ref()
                .unwrap()
                .contains("OrganizationId")
        );
    }
}
