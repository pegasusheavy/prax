import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-triggers-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-triggers.page.html',
})
export class QueriesTriggersPage {
  basicTrigger = `use prax::trigger::{Trigger, TriggerTiming, TriggerEvent, TriggerLevel};

// Create a basic audit trigger
let trigger = Trigger::builder()
    .name("audit_users")
    .table("users")
    .timing(TriggerTiming::After)
    .events([TriggerEvent::Insert, TriggerEvent::Update, TriggerEvent::Delete])
    .level(TriggerLevel::Row)
    .body(r#"
        INSERT INTO audit_log (table_name, operation, old_data, new_data, changed_at)
        VALUES ('users', TG_OP, row_to_json(OLD), row_to_json(NEW), NOW());
    "#)
    .build();

// Generate SQL for PostgreSQL
let sql = trigger.to_postgres_sql();
// CREATE TRIGGER audit_users
// AFTER INSERT OR UPDATE OR DELETE ON users
// FOR EACH ROW
// EXECUTE FUNCTION audit_users_fn();`;

  conditionalTrigger = `use prax::trigger::{Trigger, TriggerCondition};

// Trigger with WHEN condition
let trigger = Trigger::builder()
    .name("notify_price_change")
    .table("products")
    .timing(TriggerTiming::After)
    .events([TriggerEvent::Update])
    .level(TriggerLevel::Row)
    .condition(TriggerCondition::when("OLD.price <> NEW.price"))
    .body(r#"
        PERFORM pg_notify('price_changes', json_build_object(
            'product_id', NEW.id,
            'old_price', OLD.price,
            'new_price', NEW.price
        )::text);
    "#)
    .build();

// UPDATE OF specific columns (PostgreSQL)
let trigger = Trigger::builder()
    .name("track_email_changes")
    .table("users")
    .timing(TriggerTiming::After)
    .events([TriggerEvent::Update])
    .update_of(["email", "phone"])  // Only fires for these columns
    .level(TriggerLevel::Row)
    .body("...")
    .build();`;

  insteadOfTrigger = `use prax::trigger::{Trigger, TriggerTiming};

// INSTEAD OF trigger for updatable views
let trigger = Trigger::builder()
    .name("update_user_profile_view")
    .table("user_profile_view")  // A view, not a table
    .timing(TriggerTiming::InsteadOf)
    .events([TriggerEvent::Update])
    .level(TriggerLevel::Row)
    .body(r#"
        UPDATE users SET name = NEW.name WHERE id = NEW.user_id;
        UPDATE profiles SET bio = NEW.bio WHERE user_id = NEW.user_id;
        RETURN NEW;
    "#)
    .build();

// Supported on PostgreSQL, SQLite, MSSQL (not MySQL)`;

  triggerPatterns = `use prax::trigger::patterns;

// Pre-built audit trail trigger
let audit = patterns::audit_trigger("orders", "order_audit_log");
// Tracks all changes with user info, timestamp, and diff

// Soft delete trigger
let soft_delete = patterns::soft_delete_trigger("users");
// Converts DELETE to UPDATE SET deleted_at = NOW()

// Updated_at trigger
let updated_at = patterns::updated_at_trigger("posts", "updated_at");
// Auto-updates timestamp on any modification

// Validation trigger
let validation = patterns::validation_trigger("orders")
    .check("total > 0", "Order total must be positive")
    .check("status IN ('pending', 'confirmed', 'shipped')", "Invalid status")
    .build();`;

  mongoChangeStream = `use prax::trigger::{ChangeStreamBuilder, ChangeType, ChangeStreamOptions};
use futures::StreamExt;

// MongoDB Change Streams (trigger equivalent)
let mut stream = ChangeStreamBuilder::new("users")
    .watch_events([ChangeType::Insert, ChangeType::Update, ChangeType::Delete])
    .full_document(FullDocumentType::UpdateLookup)
    .full_document_before_change(true)  // Requires MongoDB 6.0+
    .resume_after(last_resume_token)    // For resumability
    .build(&client)
    .await?;

// Process changes asynchronously
while let Some(event) = stream.next().await {
    let change = event?;

    match change.operation_type {
        ChangeType::Insert => {
            let doc = change.full_document.unwrap();
            send_welcome_email(&doc).await?;
        }
        ChangeType::Update => {
            let before = change.full_document_before_change;
            let after = change.full_document;
            log_changes(before, after).await?;
        }
        ChangeType::Delete => {
            let key = change.document_key;
            cleanup_related_data(&key).await?;
        }
        _ => {}
    }

    // Save resume token for crash recovery
    save_resume_token(change.resume_token).await?;
}`;

  eventScheduler = `use prax_migrate::procedure::{ScheduledEvent, EventSchedule, EventInterval};

// MySQL Event Scheduler
let cleanup_event = ScheduledEvent::new("cleanup_old_sessions")
    .schedule(EventSchedule::every(EventInterval::Hours(1)))
    .body("DELETE FROM sessions WHERE expires_at < NOW()")
    .enabled(true)
    .preserve(false);  // Don't keep after completion

// One-time event
let report = ScheduledEvent::new("monthly_report")
    .schedule(EventSchedule::at("2024-02-01 00:00:00"))
    .body("CALL generate_monthly_report('2024-01')")
    .preserve(true);

// Complex schedule
let daily_cleanup = ScheduledEvent::new("daily_maintenance")
    .schedule(
        EventSchedule::every(EventInterval::Days(1))
            .starts("2024-01-01 02:00:00")
            .ends("2024-12-31 23:59:59")
    )
    .body(r#"
        BEGIN
            DELETE FROM logs WHERE created_at < DATE_SUB(NOW(), INTERVAL 30 DAY);
            OPTIMIZE TABLE logs;
        END
    "#);`;

  sqlAgentJob = `use prax_migrate::procedure::{SqlAgentJob, JobStep, StepType, JobSchedule, Weekday};

// MSSQL SQL Server Agent Job
let job = SqlAgentJob::new("weekly_data_cleanup")
    .description("Cleanup old data and rebuild indexes")
    .owner("sa")
    .enabled(true)
    // Step 1: Delete old records
    .add_step(
        JobStep::new("delete_old_logs")
            .step_type(StepType::TSql)
            .database("production")
            .command(r#"
                DELETE FROM dbo.AuditLogs
                WHERE CreatedAt < DATEADD(month, -6, GETDATE())
            "#)
            .on_success_action(JobStepAction::GoToNextStep)
            .on_fail_action(JobStepAction::QuitWithFailure)
    )
    // Step 2: Rebuild indexes
    .add_step(
        JobStep::new("rebuild_indexes")
            .step_type(StepType::TSql)
            .database("production")
            .command("EXEC dbo.RebuildAllIndexes")
    )
    // Step 3: Send notification
    .add_step(
        JobStep::new("send_notification")
            .step_type(StepType::CmdExec)
            .command("powershell.exe -File C:\\\\Scripts\\\\SendReport.ps1")
    )
    // Schedule: Every Sunday at 2 AM
    .schedule(
        JobSchedule::weekly()
            .on_days([Weekday::Sunday])
            .at_time("02:00:00")
            .name("weekly_sunday_schedule")
    );`;

  atlasTrigger = `use prax_migrate::procedure::{AtlasTrigger, AtlasTriggerType, AtlasOperation};

// MongoDB Atlas Database Trigger
let user_signup = AtlasTrigger::new("onUserSignup")
    .trigger_type(AtlasTriggerType::Database)
    .database("production")
    .collection("users")
    .operations([AtlasOperation::Insert])
    .full_document(true)
    .function_name("sendWelcomeEmail");

// Scheduled Trigger (Atlas Functions)
let daily_report = AtlasTrigger::new("dailyAnalytics")
    .trigger_type(AtlasTriggerType::Scheduled)
    .schedule("0 0 * * *")  // Cron: midnight daily
    .function_name("generateDailyReport");

// Authentication Trigger
let on_login = AtlasTrigger::new("onUserLogin")
    .trigger_type(AtlasTriggerType::Authentication)
    .operation_type("LOGIN")
    .function_name("updateLastLogin");

// The function runs in Atlas:
// exports = async function(changeEvent) {
//   const user = changeEvent.fullDocument;
//   await context.services.get("email").send({
//     to: user.email,
//     subject: "Welcome!",
//     body: \`Hello \${user.name}!\`
//   });
// };`;

  triggerMigration = `use prax_migrate::procedure::{TriggerDefinition, ProcedureDiffer, ProcedureSqlGenerator};

// Define triggers for migration tracking
let trigger = TriggerDefinition::new("audit_orders")
    .table("orders")
    .timing(TriggerTiming::After)
    .events([TriggerEvent::Insert, TriggerEvent::Update, TriggerEvent::Delete])
    .level(TriggerLevel::Row)
    .body("INSERT INTO audit_log ...");

// Diff triggers between schema versions
let differ = ProcedureDiffer::new();
let changes = differ.diff_triggers(&old_triggers, &new_triggers);

// Generate migration SQL
let generator = ProcedureSqlGenerator::new(DatabaseType::PostgreSQL);
for change in changes {
    match change {
        TriggerChange::Added(t) => {
            let sql = generator.create_trigger(&t);
            migration.add_up(sql);
            migration.add_down(generator.drop_trigger(&t));
        }
        TriggerChange::Modified { old, new } => {
            // PostgreSQL: DROP + CREATE (no ALTER TRIGGER for body)
            migration.add_up(generator.drop_trigger(&old));
            migration.add_up(generator.create_trigger(&new));
        }
        TriggerChange::Removed(t) => {
            migration.add_up(generator.drop_trigger(&t));
            migration.add_down(generator.create_trigger(&t));
        }
    }
}`;
}
