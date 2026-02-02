import { PGlite } from '@electric-sql/pglite';
import { drizzle } from 'drizzle-orm/pglite';

/**
 * Creates an in-memory test database with users and sessions tables.
 * Returns both the PGlite client and Drizzle ORM instance.
 */
export async function createTestDb() {
  const client = new PGlite();

  // Create users table
  await client.exec(`
    CREATE TABLE "users" (
      "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
      "github_id" integer NOT NULL,
      "username" varchar(255) NOT NULL,
      "display_name" varchar(255),
      "avatar_url" text,
      "access_token" text NOT NULL,
      "created_at" timestamp DEFAULT now() NOT NULL,
      "updated_at" timestamp DEFAULT now() NOT NULL
    );
  `);

  // Create sessions table with foreign key to users
  await client.exec(`
    CREATE TABLE "sessions" (
      "id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
      "user_id" uuid NOT NULL,
      "expires_at" timestamp NOT NULL,
      "created_at" timestamp DEFAULT now() NOT NULL
    );
  `);

  // Add foreign key constraint
  await client.exec(`
    ALTER TABLE "sessions"
    ADD CONSTRAINT "sessions_user_id_users_id_fk"
    FOREIGN KEY ("user_id")
    REFERENCES "public"."users"("id")
    ON DELETE cascade
    ON UPDATE no action;
  `);

  // Create unique index on github_id
  await client.exec(`
    CREATE UNIQUE INDEX "users_github_id_idx"
    ON "users" USING btree ("github_id");
  `);

  // Create index on sessions user_id
  await client.exec(`
    CREATE INDEX "sessions_user_id_idx"
    ON "sessions" USING btree ("user_id");
  `);

  const db = drizzle(client);

  return { db, client };
}

/**
 * Closes the PGlite database connection.
 */
export async function cleanupTestDb(client: PGlite) {
  await client.close();
}
