import { createInsertSchema, createSelectSchema } from 'drizzle-zod'
import {
  users,
  subscriptions,
  githubEvents,
  cachedIssues,
  aiDrafts,
  repoAiInsights,
  repoScores,
  issueScores,
} from './schema'

// ─── Users ───────────────────────────────────────────────────────────────────

export const insertUserSchema = createInsertSchema(users)
export const selectUserSchema = createSelectSchema(users)

// ─── Subscriptions ───────────────────────────────────────────────────────────

export const insertSubscriptionSchema = createInsertSchema(subscriptions)
export const selectSubscriptionSchema = createSelectSchema(subscriptions)

// ─── GitHub Events ───────────────────────────────────────────────────────────

export const insertGithubEventSchema = createInsertSchema(githubEvents)
export const selectGithubEventSchema = createSelectSchema(githubEvents)

// ─── Cached Issues ───────────────────────────────────────────────────────────

export const insertCachedIssueSchema = createInsertSchema(cachedIssues)
export const selectCachedIssueSchema = createSelectSchema(cachedIssues)

// ─── AI Drafts ───────────────────────────────────────────────────────────────

export const insertAiDraftSchema = createInsertSchema(aiDrafts)
export const selectAiDraftSchema = createSelectSchema(aiDrafts)

// ─── Repo AI Insights ────────────────────────────────────────────────────────

export const insertRepoAiInsightSchema = createInsertSchema(repoAiInsights)
export const selectRepoAiInsightSchema = createSelectSchema(repoAiInsights)

// ─── Repo Scores ─────────────────────────────────────────────────────────────

export const insertRepoScoreSchema = createInsertSchema(repoScores)
export const selectRepoScoreSchema = createSelectSchema(repoScores)

// ─── Issue Scores ────────────────────────────────────────────────────────────

export const insertIssueScoreSchema = createInsertSchema(issueScores)
export const selectIssueScoreSchema = createSelectSchema(issueScores)
