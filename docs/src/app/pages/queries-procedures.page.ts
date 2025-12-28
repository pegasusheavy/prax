import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-queries-procedures-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './queries-procedures.page.html',
})
export class QueriesProceduresPage {
  basicCall = `use prax::procedure::{ProcedureCall, Parameter, ParameterMode};

// Call a stored procedure
let result = ProcedureCall::new("get_user_orders")
    .param("user_id", 123)
    .param("status", "completed")
    .exec(&client)
    .await?;

// Access result sets
for row in result.rows() {
    let order_id: i32 = row.get("order_id")?;
    let total: Decimal = row.get("total")?;
    println!("Order {}: ${}", order_id, total);
}`;

  outParameters = `use prax::procedure::{ProcedureCall, Parameter, ParameterMode, ProcedureResult};

// Procedure with OUT parameters
let result = ProcedureCall::new("calculate_totals")
    .param(Parameter::new("order_id", 123).mode(ParameterMode::In))
    .param(Parameter::new("subtotal", 0.0).mode(ParameterMode::Out))
    .param(Parameter::new("tax", 0.0).mode(ParameterMode::Out))
    .param(Parameter::new("total", 0.0).mode(ParameterMode::Out))
    .exec(&client)
    .await?;

// Access output values
let subtotal: f64 = result.output("subtotal")?;
let tax: f64 = result.output("tax")?;
let total: f64 = result.output("total")?;

println!("Subtotal: ${}, Tax: ${}, Total: ${}", subtotal, tax, total);

// INOUT parameters (value modified in-place)
let result = ProcedureCall::new("increment_counter")
    .param(Parameter::new("counter", 10).mode(ParameterMode::InOut))
    .exec(&client)
    .await?;

let new_value: i32 = result.output("counter")?;`;

  functionCall = `use prax::procedure::ProcedureCall;

// Call a user-defined function
let distance = ProcedureCall::function("calculate_distance")
    .param("lat1", 40.7128)
    .param("lon1", -74.0060)
    .param("lat2", 34.0522)
    .param("lon2", -118.2437)
    .exec(&client)
    .await?
    .scalar::<f64>()?;

println!("Distance: {} km", distance);

// Table-valued function (PostgreSQL/MSSQL)
let nearby = ProcedureCall::table_function("find_nearby_stores")
    .param("lat", 40.7128)
    .param("lon", -74.0060)
    .param("radius_km", 10.0)
    .exec(&client)
    .await?;

for store in nearby.rows() {
    let name: String = store.get("name")?;
    let distance: f64 = store.get("distance")?;
    println!("{}: {} km away", name, distance);
}`;

  sqliteUdf = `use prax::procedure::sqlite_udf::{ScalarUdf, AggregateUdf, WindowUdf};

// Register a scalar UDF in SQLite
let levenshtein = ScalarUdf::new("levenshtein")
    .args(2)  // Two string arguments
    .deterministic(true)
    .handler(|args| {
        let s1: &str = args.get(0)?;
        let s2: &str = args.get(1)?;
        Ok(levenshtein_distance(s1, s2) as i64)
    });

client.register_function(levenshtein)?;

// Use in queries
let similar = client.raw_query(
    "SELECT * FROM products WHERE levenshtein(name, ?) < 3",
    ["headphones"]
).await?;

// Register an aggregate UDF
let median = AggregateUdf::new("median")
    .init(|| Vec::new())
    .step(|state: &mut Vec<f64>, value: f64| {
        state.push(value);
    })
    .finalize(|mut state: Vec<f64>| {
        state.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = state.len() / 2;
        if state.len() % 2 == 0 {
            (state[mid - 1] + state[mid]) / 2.0
        } else {
            state[mid]
        }
    });

client.register_aggregate(median)?;

// Window UDF
let running_avg = WindowUdf::new("running_avg")
    .value_handler(|partition, current_row| {
        let sum: f64 = partition[..=current_row].iter().sum();
        sum / (current_row + 1) as f64
    });`;

  mongoFunction = `use prax::procedure::mongodb::{MongoFunction, MongoAccumulator};

// MongoDB $function (JavaScript UDF)
let custom_fn = MongoFunction::new("calculateScore")
    .args(["score", "multiplier", "bonus"])
    .body(r#"
        function(score, multiplier, bonus) {
            return (score * multiplier) + bonus;
        }
    "#)
    .lang("js");

// Use in aggregation pipeline
let pipeline = vec![
    doc! {
        "$addFields": {
            "finalScore": custom_fn.to_bson()
        }
    }
];

// MongoDB $accumulator (custom aggregation)
let weighted_avg = MongoAccumulator::new("weightedAverage")
    .init_args(["weights"])
    .init(r#"
        function(weights) {
            return { sum: 0, weightSum: 0, weights: weights };
        }
    "#)
    .accumulate_args(["state", "value", "index"])
    .accumulate(r#"
        function(state, value, index) {
            const weight = state.weights[index] || 1;
            state.sum += value * weight;
            state.weightSum += weight;
            return state;
        }
    "#)
    .merge(r#"
        function(state1, state2) {
            return {
                sum: state1.sum + state2.sum,
                weightSum: state1.weightSum + state2.weightSum,
                weights: state1.weights
            };
        }
    "#)
    .finalize(r#"
        function(state) {
            return state.sum / state.weightSum;
        }
    "#);`;

  procedureMigration = `use prax_migrate::procedure::{
    ProcedureDefinition, ProcedureParameter, ProcedureLanguage,
    ProcedureVolatility, ProcedureSecurity, ProcedureDiffer, ProcedureSqlGenerator
};

// Define a stored procedure for migration
let procedure = ProcedureDefinition::new("get_user_orders")
    .language(ProcedureLanguage::PlPgSql)
    .param(ProcedureParameter::new("p_user_id", "INT").mode(ParameterMode::In))
    .param(ProcedureParameter::new("p_status", "VARCHAR(50)").mode(ParameterMode::In))
    .returns("TABLE(order_id INT, total DECIMAL, created_at TIMESTAMP)")
    .volatility(ProcedureVolatility::Stable)
    .security(ProcedureSecurity::Definer)
    .body(r#"
        BEGIN
            RETURN QUERY
            SELECT o.id, o.total, o.created_at
            FROM orders o
            WHERE o.user_id = p_user_id
              AND (p_status IS NULL OR o.status = p_status)
            ORDER BY o.created_at DESC;
        END;
    "#);

// Generate SQL for different databases
let postgres_sql = ProcedureSqlGenerator::new(DatabaseType::PostgreSQL)
    .create_procedure(&procedure);

let mysql_sql = ProcedureSqlGenerator::new(DatabaseType::MySQL)
    .create_procedure(&procedure);

// Diff procedures for migrations
let differ = ProcedureDiffer::new();
let changes = differ.diff(&old_procedures, &new_procedures);

for change in changes {
    match change {
        ProcedureChange::Added(proc) => {
            println!("CREATE: {}", proc.name);
        }
        ProcedureChange::Modified { old, new } => {
            println!("ALTER: {}", new.name);
        }
        ProcedureChange::Removed(proc) => {
            println!("DROP: {}", proc.name);
        }
    }
}`;

  databaseComparison = `// PostgreSQL procedure
CREATE OR REPLACE FUNCTION get_user_orders(
    p_user_id INT,
    p_status VARCHAR(50) DEFAULT NULL
) RETURNS TABLE(order_id INT, total DECIMAL, created_at TIMESTAMP)
LANGUAGE plpgsql STABLE SECURITY DEFINER
AS $$
BEGIN
    RETURN QUERY SELECT ...;
END;
$$;

-- MySQL procedure
DELIMITER //
CREATE PROCEDURE get_user_orders(
    IN p_user_id INT,
    IN p_status VARCHAR(50)
)
BEGIN
    SELECT o.id, o.total, o.created_at
    FROM orders o
    WHERE o.user_id = p_user_id
      AND (p_status IS NULL OR o.status = p_status);
END //
DELIMITER ;

-- MSSQL procedure
CREATE OR ALTER PROCEDURE get_user_orders
    @p_user_id INT,
    @p_status VARCHAR(50) = NULL
AS
BEGIN
    SELECT o.id, o.total, o.created_at
    FROM orders o
    WHERE o.user_id = @p_user_id
      AND (@p_status IS NULL OR o.status = @p_status);
END;`;
}




