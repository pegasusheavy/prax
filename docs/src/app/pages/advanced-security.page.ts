import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-advanced-security-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './advanced-security.page.html',
})
export class AdvancedSecurityPage {
  rlsPolicy = `use prax::security::{RlsPolicy, RlsPolicyBuilder};

// Row-Level Security policy (PostgreSQL/MSSQL)
let tenant_policy = RlsPolicyBuilder::new("tenant_isolation")
    .table("orders")
    .for_all()  // SELECT, INSERT, UPDATE, DELETE
    .using("tenant_id = current_setting('app.tenant_id')::int")
    .with_check("tenant_id = current_setting('app.tenant_id')::int")
    .build();

// Generated SQL (PostgreSQL):
// ALTER TABLE orders ENABLE ROW LEVEL SECURITY;
// CREATE POLICY tenant_isolation ON orders
//   FOR ALL
//   USING (tenant_id = current_setting('app.tenant_id')::int)
//   WITH CHECK (tenant_id = current_setting('app.tenant_id')::int);

// Permissive policy (OR'd with other permissive policies)
let admin_policy = RlsPolicyBuilder::new("admin_access")
    .table("orders")
    .for_select()
    .permissive()
    .using("current_setting('app.role') = 'admin'")
    .build();

// Restrictive policy (AND'd with other policies)
let active_only = RlsPolicyBuilder::new("active_only")
    .table("orders")
    .restrictive()
    .using("status != 'deleted'")
    .build();`;

  multiTenant = `use prax::security::{TenantPolicy, TenantSource};

// Multi-tenant RLS setup
let tenant = TenantPolicy::new("tenant_id")
    .source(TenantSource::SessionVariable("app.tenant_id"))
    .tables(["users", "orders", "products", "invoices"])
    .build();

// Using in queries - set session variable first
client.raw_execute(
    "SET app.tenant_id = $1",
    [&tenant_id]
).await?;

// All subsequent queries automatically filtered
let orders = client
    .order()
    .find_many()
    .exec()
    .await?;  // Only returns orders for current tenant

// Alternative: JWT claim-based tenancy
let jwt_tenant = TenantPolicy::new("tenant_id")
    .source(TenantSource::JwtClaim("https://myapp.com/tenant_id"))
    .build();`;

  roleManagement = `use prax::security::{Role, RoleBuilder, Privilege};

// Create application roles
let reader = RoleBuilder::new("app_reader")
    .login(false)
    .inherit(true)
    .build();

let writer = RoleBuilder::new("app_writer")
    .login(false)
    .inherit(true)
    .member_of(["app_reader"])
    .build();

let admin = RoleBuilder::new("app_admin")
    .login(false)
    .inherit(true)
    .member_of(["app_writer"])
    .createdb(false)
    .createrole(false)
    .build();

// Create login role
let api_user = RoleBuilder::new("api_service")
    .login(true)
    .password("secure_password")
    .member_of(["app_writer"])
    .connection_limit(10)
    .valid_until("2025-12-31")
    .build();

// PostgreSQL:
// CREATE ROLE app_reader NOLOGIN INHERIT;
// CREATE ROLE app_writer NOLOGIN INHERIT IN ROLE app_reader;
// CREATE ROLE app_admin NOLOGIN INHERIT IN ROLE app_writer;
// CREATE ROLE api_service LOGIN PASSWORD 'xxx' IN ROLE app_writer
//   CONNECTION LIMIT 10 VALID UNTIL '2025-12-31';`;

  grants = `use prax::security::{Grant, GrantBuilder, Privilege};

// Table-level grants
let table_grant = GrantBuilder::new()
    .privileges([Privilege::Select, Privilege::Insert, Privilege::Update])
    .on_table("orders")
    .to_role("app_writer")
    .build();

// Column-level grants (fine-grained access)
let column_grant = GrantBuilder::new()
    .privileges([Privilege::Select])
    .on_columns(["email", "name"])
    .of_table("users")
    .to_role("support_staff")
    .build();

// Exclude sensitive columns
let restricted = GrantBuilder::new()
    .privileges([Privilege::Select])
    .on_table("users")
    .excluding_columns(["password_hash", "ssn", "salary"])
    .to_role("app_reader")
    .build();

// Schema grants
let schema_grant = GrantBuilder::new()
    .privileges([Privilege::Usage])
    .on_schema("api")
    .to_role("api_service")
    .build();

// WITH GRANT OPTION (allow grantee to grant to others)
let delegated = GrantBuilder::new()
    .privileges([Privilege::Select])
    .on_table("public_data")
    .to_role("data_admin")
    .with_grant_option()
    .build();`;

  dataMasking = `use prax::security::{DataMask, MaskFunction};

// Dynamic data masking (MSSQL / PostgreSQL with extension)
let email_mask = DataMask::new("users", "email")
    .function(MaskFunction::Email)  // Shows first char + @domain
    .build();

// Partial masking
let phone_mask = DataMask::new("customers", "phone")
    .function(MaskFunction::Partial {
        prefix_len: 0,
        suffix_len: 4,
        mask_char: 'X',
    })  // XXXXXXX1234
    .build();

// Full masking
let ssn_mask = DataMask::new("employees", "ssn")
    .function(MaskFunction::Default)  // Shows 'xxxx'
    .build();

// Custom masking function
let custom = DataMask::new("orders", "total")
    .function(MaskFunction::Custom("ROUND(total, -2)"))
    .build();

// Grant UNMASK permission to specific roles
let unmask = GrantBuilder::new()
    .privileges([Privilege::Unmask])
    .on_table("users")
    .to_role("data_admin")
    .build();`;

  connectionProfile = `use prax::security::ConnectionProfile;

// Named connection profiles with security settings
let readonly = ConnectionProfile::new("readonly")
    .role("app_reader")
    .search_path(["public", "api"])
    .statement_timeout("30s")
    .lock_timeout("5s")
    .read_only(true)
    .row_security(true)
    .build();

let api_profile = ConnectionProfile::new("api")
    .role("api_service")
    .set("app.tenant_id", &tenant_id)
    .set("app.user_id", &user_id)
    .search_path(["tenant_schema", "public"])
    .statement_timeout("60s")
    .build();

// Apply profile to connection
let conn = client
    .connection()
    .with_profile(api_profile)
    .await?;

// Execute with profile
let orders = conn.order().find_many().exec().await?;`;

  mongoSecurity = `use prax::security::mongodb::{MongoRole, FieldEncryption, KmsProvider};

// MongoDB role-based access control
let analyst = MongoRole::new("analyst")
    .database("analytics")
    .privileges([
        ("reports", ["find", "aggregate"]),
        ("dashboards", ["find"]),
    ])
    .inherited_roles([("read", "reporting")])
    .build();

// Client-Side Field Level Encryption (CSFLE)
let encryption = FieldEncryption::new()
    // AWS KMS
    .kms_provider(KmsProvider::Aws {
        access_key_id: env!("AWS_ACCESS_KEY_ID"),
        secret_access_key: env!("AWS_SECRET_ACCESS_KEY"),
        region: "us-east-1",
    })
    // Or Azure Key Vault
    // .kms_provider(KmsProvider::Azure {
    //     tenant_id: "...",
    //     client_id: "...",
    //     client_secret: "...",
    //     key_vault_endpoint: "https://myvault.vault.azure.net",
    // })
    .key_vault_namespace("encryption.__keyVault")
    .schema_map("users", doc! {
        "bsonType": "object",
        "encryptMetadata": {
            "keyId": [data_key_id]
        },
        "properties": {
            "ssn": {
                "encrypt": {
                    "bsonType": "string",
                    "algorithm": "AEAD_AES_256_CBC_HMAC_SHA_512-Deterministic"
                }
            },
            "medical_records": {
                "encrypt": {
                    "bsonType": "array",
                    "algorithm": "AEAD_AES_256_CBC_HMAC_SHA_512-Random"
                }
            }
        }
    });

let secure_client = PraxClient::mongodb()
    .url(db_url)
    .encryption(encryption)
    .connect()
    .await?;`;
}



