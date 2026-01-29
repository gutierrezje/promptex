CREATE TABLE "ai_drafts" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"issue_id" uuid,
	"repo_owner" varchar(255) NOT NULL,
	"repo_name" varchar(255) NOT NULL,
	"issue_number" integer NOT NULL,
	"draft_type" varchar(30) NOT NULL,
	"content" text NOT NULL,
	"status" varchar(20) DEFAULT 'pending' NOT NULL,
	"quality_rating" varchar(10),
	"quality_assessment" jsonb,
	"model_used" varchar(100),
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "cached_issues" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"repo_owner" varchar(255) NOT NULL,
	"repo_name" varchar(255) NOT NULL,
	"issue_number" integer NOT NULL,
	"title" text NOT NULL,
	"body" text,
	"state" varchar(20) NOT NULL,
	"author_username" varchar(255),
	"labels" jsonb,
	"assignees" jsonb,
	"comment_count" integer DEFAULT 0,
	"github_url" text,
	"created_at_gh" timestamp,
	"updated_at_gh" timestamp,
	"synced_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "github_events" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"github_event_id" varchar(255),
	"repo_owner" varchar(255) NOT NULL,
	"repo_name" varchar(255) NOT NULL,
	"event_type" varchar(50) NOT NULL,
	"title" text,
	"body" text,
	"actor_username" varchar(255),
	"actor_avatar" text,
	"metadata" jsonb,
	"github_url" text,
	"occurred_at" timestamp NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "issue_scores" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"repo_owner" varchar(255) NOT NULL,
	"repo_name" varchar(255) NOT NULL,
	"issue_number" integer NOT NULL,
	"labels" jsonb,
	"has_good_first_issue" boolean DEFAULT false NOT NULL,
	"has_help_wanted" boolean DEFAULT false NOT NULL,
	"comment_count" integer DEFAULT 0 NOT NULL,
	"linked_pr_count" integer DEFAULT 0 NOT NULL,
	"days_since_last_activity" integer DEFAULT 0 NOT NULL,
	"has_repro" boolean,
	"has_clear_description" boolean,
	"contribution_fit_score" smallint DEFAULT 0 NOT NULL,
	"last_scored_at" timestamp DEFAULT now() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "repo_ai_insights" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"repo_owner" varchar(255) NOT NULL,
	"repo_name" varchar(255) NOT NULL,
	"insight_type" varchar(50) NOT NULL,
	"target_issue_number" integer,
	"content" jsonb NOT NULL,
	"model_used" varchar(100),
	"generated_at" timestamp DEFAULT now() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "repo_scores" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"repo_owner" varchar(255) NOT NULL,
	"repo_name" varchar(255) NOT NULL,
	"stars" integer DEFAULT 0 NOT NULL,
	"open_issues_count" integer DEFAULT 0 NOT NULL,
	"contributor_count" integer DEFAULT 0 NOT NULL,
	"avg_issue_response_hours" real,
	"maintainer_activity_score" smallint DEFAULT 0 NOT NULL,
	"contribution_opportunity_score" smallint DEFAULT 0 NOT NULL,
	"last_scored_at" timestamp DEFAULT now() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "sessions" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"expires_at" timestamp NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE "subscriptions" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"user_id" uuid NOT NULL,
	"repo_owner" varchar(255) NOT NULL,
	"repo_name" varchar(255) NOT NULL,
	"webhook_id" text,
	"webhook_secret" text,
	"last_synced_at" timestamp,
	"created_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
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
--> statement-breakpoint
ALTER TABLE "ai_drafts" ADD CONSTRAINT "ai_drafts_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "ai_drafts" ADD CONSTRAINT "ai_drafts_issue_id_cached_issues_id_fk" FOREIGN KEY ("issue_id") REFERENCES "public"."cached_issues"("id") ON DELETE no action ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "sessions" ADD CONSTRAINT "sessions_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
ALTER TABLE "subscriptions" ADD CONSTRAINT "subscriptions_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE cascade ON UPDATE no action;--> statement-breakpoint
CREATE INDEX "ai_drafts_user_issue_idx" ON "ai_drafts" USING btree ("user_id","issue_id");--> statement-breakpoint
CREATE UNIQUE INDEX "cached_issues_repo_number_idx" ON "cached_issues" USING btree ("repo_owner","repo_name","issue_number");--> statement-breakpoint
CREATE INDEX "cached_issues_state_idx" ON "cached_issues" USING btree ("repo_owner","repo_name","state");--> statement-breakpoint
CREATE UNIQUE INDEX "github_events_event_id_idx" ON "github_events" USING btree ("github_event_id");--> statement-breakpoint
CREATE INDEX "github_events_repo_time_idx" ON "github_events" USING btree ("repo_owner","repo_name","occurred_at");--> statement-breakpoint
CREATE UNIQUE INDEX "issue_scores_repo_issue_idx" ON "issue_scores" USING btree ("repo_owner","repo_name","issue_number");--> statement-breakpoint
CREATE INDEX "issue_scores_fit_idx" ON "issue_scores" USING btree ("contribution_fit_score");--> statement-breakpoint
CREATE UNIQUE INDEX "repo_ai_insights_unique_idx" ON "repo_ai_insights" USING btree ("repo_owner","repo_name","insight_type","target_issue_number");--> statement-breakpoint
CREATE INDEX "repo_ai_insights_repo_idx" ON "repo_ai_insights" USING btree ("repo_owner","repo_name");--> statement-breakpoint
CREATE UNIQUE INDEX "repo_scores_repo_idx" ON "repo_scores" USING btree ("repo_owner","repo_name");--> statement-breakpoint
CREATE INDEX "repo_scores_opportunity_idx" ON "repo_scores" USING btree ("contribution_opportunity_score");--> statement-breakpoint
CREATE INDEX "sessions_user_id_idx" ON "sessions" USING btree ("user_id");--> statement-breakpoint
CREATE UNIQUE INDEX "subscriptions_user_repo_idx" ON "subscriptions" USING btree ("user_id","repo_owner","repo_name");--> statement-breakpoint
CREATE INDEX "subscriptions_repo_idx" ON "subscriptions" USING btree ("repo_owner","repo_name");--> statement-breakpoint
CREATE UNIQUE INDEX "users_github_id_idx" ON "users" USING btree ("github_id");