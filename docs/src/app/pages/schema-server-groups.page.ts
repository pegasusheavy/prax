import { Component } from '@angular/core';
import { CodeBlockComponent } from '../components/code-block.component';

@Component({
  selector: 'app-schema-server-groups-page',
  standalone: true,
  imports: [CodeBlockComponent],
  templateUrl: './schema-server-groups.page.html',
})
export class SchemaServerGroupsPage {
  basicServerGroup = `// Define a server group for your database cluster
serverGroup MainCluster {
    server primary {
        url = env("PRIMARY_DATABASE_URL")
        role = "primary"
    }
}`;

  readReplicaSetup = `// Read replica configuration with primary and replicas
serverGroup ProductionCluster {
    @@strategy(ReadReplica)
    @@loadBalance(RoundRobin)

    server primary {
        url = env("PRIMARY_DATABASE_URL")
        role = "primary"
        weight = 1
    }

    server replica1 {
        url = env("REPLICA1_DATABASE_URL")
        role = "replica"
        weight = 2
        region = "us-east-1"
    }

    server replica2 {
        url = env("REPLICA2_DATABASE_URL")
        role = "replica"
        weight = 2
        region = "us-west-2"
    }
}`;

  multiRegionSetup = `// Multi-region deployment for geographic distribution
serverGroup GlobalCluster {
    @@strategy(MultiRegion)
    @@loadBalance(Nearest)

    server usEast {
        url = env("US_EAST_DATABASE_URL")
        role = "primary"
        region = "us-east-1"
        priority = 1
    }

    server euWest {
        url = env("EU_WEST_DATABASE_URL")
        role = "replica"
        region = "eu-west-1"
        priority = 2
    }

    server apSouth {
        url = env("AP_SOUTH_DATABASE_URL")
        role = "replica"
        region = "ap-southeast-1"
        priority = 3
    }
}`;

  highAvailability = `// High availability configuration with automatic failover
serverGroup HACluster {
    @@strategy(HighAvailability)

    server primary {
        url = env("PRIMARY_URL")
        role = "primary"
        priority = 1
        healthCheck = "/health"
        maxConnections = 100
    }

    server standby1 {
        url = env("STANDBY1_URL")
        role = "replica"
        priority = 2
        healthCheck = "/health"
    }

    server standby2 {
        url = env("STANDBY2_URL")
        role = "replica"
        priority = 3
        healthCheck = "/health"
    }
}`;

  shardingSetup = `// Sharding configuration for horizontal scaling
serverGroup ShardedCluster {
    @@strategy(Sharding)

    server shard1 {
        url = env("SHARD1_URL")
        role = "shard"
        shardKey = "user_id"
        shardRange = "0-999999"
    }

    server shard2 {
        url = env("SHARD2_URL")
        role = "shard"
        shardKey = "user_id"
        shardRange = "1000000-1999999"
    }

    server shard3 {
        url = env("SHARD3_URL")
        role = "shard"
        shardKey = "user_id"
        shardRange = "2000000-2999999"
    }
}`;

  analyticsSetup = `// Separate analytics server for reporting
serverGroup DataCluster {
    @@strategy(Custom)

    server primary {
        url = env("PRIMARY_DATABASE_URL")
        role = "primary"
    }

    server analytics {
        url = env("ANALYTICS_DATABASE_URL")
        role = "analytics"
        readOnly = true
    }

    server archive {
        url = env("ARCHIVE_DATABASE_URL")
        role = "archive"
        readOnly = true
    }
}`;

  weightedLoadBalancing = `// Weighted load balancing for uneven server capacity
serverGroup WeightedCluster {
    @@strategy(ReadReplica)
    @@loadBalance(Weighted)

    server primary {
        url = env("PRIMARY_URL")
        role = "primary"
        weight = 1    // Receives 1/6 of reads
    }

    server powerful {
        url = env("POWERFUL_REPLICA_URL")
        role = "replica"
        weight = 3    // Receives 3/6 of reads (50%)
    }

    server standard {
        url = env("STANDARD_REPLICA_URL")
        role = "replica"
        weight = 2    // Receives 2/6 of reads (~33%)
    }
}`;

  fullExample = `// Complete schema with server groups and models
serverGroup Production {
    @@strategy(ReadReplica)
    @@loadBalance(RoundRobin)

    server primary {
        url = env("DATABASE_URL")
        role = "primary"
        maxConnections = 50
    }

    server replica {
        url = env("REPLICA_URL")
        role = "replica"
        weight = 2
    }
}

model User {
    id        Int      @id @auto
    email     String   @unique
    name      String?
    posts     Post[]
    createdAt DateTime @default(now())
}

model Post {
    id        Int      @id @auto
    title     String
    content   String?
    author    User     @relation(fields: [authorId], references: [id])
    authorId  Int      @map("author_id")
    createdAt DateTime @default(now())
}`;

  rustUsage = `// Server groups are configured at runtime
use prax::{PraxClient, ServerGroupConfig};

let config = ServerGroupConfig::from_schema("Production")
    .with_read_write_splitting(true)
    .with_health_check_interval(Duration::from_secs(30));

let client = PraxClient::with_server_group(config).await?;

// Writes automatically go to primary
let user = client
    .user()
    .create()
    .data(data! {
        email: "user@example.com",
        name: "John"
    })
    .exec()
    .await?;

// Reads can be distributed to replicas
let users = client
    .user()
    .find_many()
    .exec()
    .await?;

// Force read from primary
let user = client
    .user()
    .find_unique()
    .where(user::id::equals(1))
    .use_primary()  // Force primary server
    .exec()
    .await?;`;
}


