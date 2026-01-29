import {
  pgTable,
  uuid,
  varchar,
  text,
  integer,
  boolean,
  timestamp,
  jsonb,
  uniqueIndex,
  index,
  smallint,
  real,
} from 'drizzle-orm/pg-core'
import { relations } from 'drizzle-orm'

// ─── Users ───────────────────────────────────────────────────────────────────

export const users = pgTable(
  'users',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    githubId: integer('github_id').notNull(),
    username: varchar('username', { length: 255 }).notNull(),
    displayName: varchar('display_name', { length: 255 }),
    avatarUrl: text('avatar_url'),
    accessToken: text('access_token').notNull(),
    createdAt: timestamp('created_at').defaultNow().notNull(),
    updatedAt: timestamp('updated_at')
      .defaultNow()
      .notNull()
      .$onUpdate(() => new Date()),
  },
  (t) => [uniqueIndex('users_github_id_idx').on(t.githubId)],
)

export const usersRelations = relations(users, ({ many }) => ({
  subscriptions: many(subscriptions),
  aiDrafts: many(aiDrafts),
}))

// ─── Sessions ────────────────────────────────────────────────────────────────

export const sessions = pgTable(
  'sessions',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    userId: uuid('user_id')
      .notNull()
      .references(() => users.id, { onDelete: 'cascade' }),
    expiresAt: timestamp('expires_at').notNull(),
    createdAt: timestamp('created_at').defaultNow().notNull(),
  },
  (t) => [index('sessions_user_id_idx').on(t.userId)],
)

export const sessionsRelations = relations(sessions, ({ one }) => ({
  user: one(users, { fields: [sessions.userId], references: [users.id] }),
}))

// ─── Subscriptions ───────────────────────────────────────────────────────────

export const subscriptions = pgTable(
  'subscriptions',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    userId: uuid('user_id')
      .notNull()
      .references(() => users.id, { onDelete: 'cascade' }),
    repoOwner: varchar('repo_owner', { length: 255 }).notNull(),
    repoName: varchar('repo_name', { length: 255 }).notNull(),
    webhookId: text('webhook_id'),
    webhookSecret: text('webhook_secret'),
    lastSyncedAt: timestamp('last_synced_at'),
    createdAt: timestamp('created_at').defaultNow().notNull(),
  },
  (t) => [
    uniqueIndex('subscriptions_user_repo_idx').on(
      t.userId,
      t.repoOwner,
      t.repoName,
    ),
    index('subscriptions_repo_idx').on(t.repoOwner, t.repoName),
  ],
)

export const subscriptionsRelations = relations(subscriptions, ({ one }) => ({
  user: one(users, {
    fields: [subscriptions.userId],
    references: [users.id],
  }),
}))

// ─── GitHub Events ───────────────────────────────────────────────────────────

export const githubEvents = pgTable(
  'github_events',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    githubEventId: varchar('github_event_id', { length: 255 }),
    repoOwner: varchar('repo_owner', { length: 255 }).notNull(),
    repoName: varchar('repo_name', { length: 255 }).notNull(),
    eventType: varchar('event_type', { length: 50 }).notNull(),
    title: text('title'),
    body: text('body'),
    actorUsername: varchar('actor_username', { length: 255 }),
    actorAvatar: text('actor_avatar'),
    metadata: jsonb('metadata').$type<Record<string, unknown>>(),
    githubUrl: text('github_url'),
    occurredAt: timestamp('occurred_at').notNull(),
    createdAt: timestamp('created_at').defaultNow().notNull(),
  },
  (t) => [
    uniqueIndex('github_events_event_id_idx').on(t.githubEventId),
    index('github_events_repo_time_idx').on(
      t.repoOwner,
      t.repoName,
      t.occurredAt,
    ),
  ],
)

// ─── Cached Issues ───────────────────────────────────────────────────────────

export const cachedIssues = pgTable(
  'cached_issues',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    repoOwner: varchar('repo_owner', { length: 255 }).notNull(),
    repoName: varchar('repo_name', { length: 255 }).notNull(),
    issueNumber: integer('issue_number').notNull(),
    title: text('title').notNull(),
    body: text('body'),
    state: varchar('state', { length: 20 }).notNull(),
    authorUsername: varchar('author_username', { length: 255 }),
    labels: jsonb('labels').$type<Array<{ name: string; color: string }>>(),
    assignees: jsonb('assignees').$type<
      Array<{ login: string; avatar_url: string }>
    >(),
    commentCount: integer('comment_count').default(0),
    githubUrl: text('github_url'),
    createdAtGh: timestamp('created_at_gh'),
    updatedAtGh: timestamp('updated_at_gh'),
    syncedAt: timestamp('synced_at').defaultNow().notNull(),
  },
  (t) => [
    uniqueIndex('cached_issues_repo_number_idx').on(
      t.repoOwner,
      t.repoName,
      t.issueNumber,
    ),
    index('cached_issues_state_idx').on(
      t.repoOwner,
      t.repoName,
      t.state,
    ),
  ],
)

// ─── AI Drafts ───────────────────────────────────────────────────────────────

export const aiDrafts = pgTable(
  'ai_drafts',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    userId: uuid('user_id')
      .notNull()
      .references(() => users.id, { onDelete: 'cascade' }),
    issueId: uuid('issue_id').references(() => cachedIssues.id),
    repoOwner: varchar('repo_owner', { length: 255 }).notNull(),
    repoName: varchar('repo_name', { length: 255 }).notNull(),
    issueNumber: integer('issue_number').notNull(),
    draftType: varchar('draft_type', { length: 30 }).notNull(),
    content: text('content').notNull(),
    status: varchar('status', { length: 20 }).default('pending').notNull(),
    qualityRating: varchar('quality_rating', { length: 10 }),
    qualityAssessment: jsonb('quality_assessment').$type<{
      valueCheck: string
      specificityCheck: string
      qualificationCheck: string
    }>(),
    modelUsed: varchar('model_used', { length: 100 }),
    createdAt: timestamp('created_at').defaultNow().notNull(),
    updatedAt: timestamp('updated_at')
      .defaultNow()
      .notNull()
      .$onUpdate(() => new Date()),
  },
  (t) => [
    index('ai_drafts_user_issue_idx').on(t.userId, t.issueId),
  ],
)

export const aiDraftsRelations = relations(aiDrafts, ({ one }) => ({
  user: one(users, { fields: [aiDrafts.userId], references: [users.id] }),
  issue: one(cachedIssues, {
    fields: [aiDrafts.issueId],
    references: [cachedIssues.id],
  }),
}))

// ─── Repo AI Insights ────────────────────────────────────────────────────────

export const repoAiInsights = pgTable(
  'repo_ai_insights',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    repoOwner: varchar('repo_owner', { length: 255 }).notNull(),
    repoName: varchar('repo_name', { length: 255 }).notNull(),
    insightType: varchar('insight_type', { length: 50 }).notNull(),
    targetIssueNumber: integer('target_issue_number'),
    content: jsonb('content').$type<Record<string, unknown>>().notNull(),
    modelUsed: varchar('model_used', { length: 100 }),
    generatedAt: timestamp('generated_at').defaultNow().notNull(),
    createdAt: timestamp('created_at').defaultNow().notNull(),
  },
  (t) => [
    uniqueIndex('repo_ai_insights_unique_idx').on(
      t.repoOwner,
      t.repoName,
      t.insightType,
      t.targetIssueNumber,
    ),
    index('repo_ai_insights_repo_idx').on(t.repoOwner, t.repoName),
  ],
)

// ─── Repo Scores ─────────────────────────────────────────────────────────────

export const repoScores = pgTable(
  'repo_scores',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    repoOwner: varchar('repo_owner', { length: 255 }).notNull(),
    repoName: varchar('repo_name', { length: 255 }).notNull(),
    stars: integer('stars').default(0).notNull(),
    openIssuesCount: integer('open_issues_count').default(0).notNull(),
    contributorCount: integer('contributor_count').default(0).notNull(),
    avgIssueResponseHours: real('avg_issue_response_hours'),
    maintainerActivityScore: smallint('maintainer_activity_score').default(0).notNull(),
    contributionOpportunityScore: smallint('contribution_opportunity_score')
      .default(0)
      .notNull(),
    lastScoredAt: timestamp('last_scored_at').defaultNow().notNull(),
    createdAt: timestamp('created_at').defaultNow().notNull(),
  },
  (t) => [
    uniqueIndex('repo_scores_repo_idx').on(t.repoOwner, t.repoName),
    index('repo_scores_opportunity_idx').on(t.contributionOpportunityScore),
  ],
)

// ─── Issue Scores ────────────────────────────────────────────────────────────

export const issueScores = pgTable(
  'issue_scores',
  {
    id: uuid('id').primaryKey().defaultRandom(),
    repoOwner: varchar('repo_owner', { length: 255 }).notNull(),
    repoName: varchar('repo_name', { length: 255 }).notNull(),
    issueNumber: integer('issue_number').notNull(),
    labels: jsonb('labels').$type<string[]>(),
    hasGoodFirstIssue: boolean('has_good_first_issue').default(false).notNull(),
    hasHelpWanted: boolean('has_help_wanted').default(false).notNull(),
    commentCount: integer('comment_count').default(0).notNull(),
    linkedPrCount: integer('linked_pr_count').default(0).notNull(),
    daysSinceLastActivity: integer('days_since_last_activity')
      .default(0)
      .notNull(),
    hasRepro: boolean('has_repro'),
    hasClearDescription: boolean('has_clear_description'),
    contributionFitScore: smallint('contribution_fit_score')
      .default(0)
      .notNull(),
    lastScoredAt: timestamp('last_scored_at').defaultNow().notNull(),
    createdAt: timestamp('created_at').defaultNow().notNull(),
  },
  (t) => [
    uniqueIndex('issue_scores_repo_issue_idx').on(
      t.repoOwner,
      t.repoName,
      t.issueNumber,
    ),
    index('issue_scores_fit_idx').on(t.contributionFitScore),
  ],
)
