import { describe, it, expect } from 'vitest'
import { getTableName, getTableColumns } from 'drizzle-orm'
import {
  users,
  sessions,
  subscriptions,
  githubEvents,
  cachedIssues,
  aiDrafts,
  repoAiInsights,
  repoScores,
  issueScores,
} from '@/server/db/schema'

describe('Database Schema', () => {
  const tables = [
    { schema: users, name: 'users' },
    { schema: sessions, name: 'sessions' },
    { schema: subscriptions, name: 'subscriptions' },
    { schema: githubEvents, name: 'github_events' },
    { schema: cachedIssues, name: 'cached_issues' },
    { schema: aiDrafts, name: 'ai_drafts' },
    { schema: repoAiInsights, name: 'repo_ai_insights' },
    { schema: repoScores, name: 'repo_scores' },
    { schema: issueScores, name: 'issue_scores' },
  ] as const

  it('exports all 9 tables', () => {
    expect(tables).toHaveLength(9)
  })

  it.each(tables)('$name table has the correct name', ({ schema, name }) => {
    expect(getTableName(schema)).toBe(name)
  })

  describe('users table', () => {
    it('has required columns', () => {
      const cols = getTableColumns(users)
      expect(cols.id).toBeDefined()
      expect(cols.githubId).toBeDefined()
      expect(cols.username).toBeDefined()
      expect(cols.accessToken).toBeDefined()
      expect(cols.createdAt).toBeDefined()
    })
  })

  describe('subscriptions table', () => {
    it('has required columns', () => {
      const cols = getTableColumns(subscriptions)
      expect(cols.userId).toBeDefined()
      expect(cols.repoOwner).toBeDefined()
      expect(cols.repoName).toBeDefined()
      expect(cols.webhookId).toBeDefined()
      expect(cols.lastSyncedAt).toBeDefined()
    })
  })

  describe('github_events table', () => {
    it('has required columns', () => {
      const cols = getTableColumns(githubEvents)
      expect(cols.githubEventId).toBeDefined()
      expect(cols.eventType).toBeDefined()
      expect(cols.repoOwner).toBeDefined()
      expect(cols.repoName).toBeDefined()
      expect(cols.occurredAt).toBeDefined()
      expect(cols.metadata).toBeDefined()
    })
  })

  describe('issue_scores table', () => {
    it('has scoring columns', () => {
      const cols = getTableColumns(issueScores)
      expect(cols.hasGoodFirstIssue).toBeDefined()
      expect(cols.hasHelpWanted).toBeDefined()
      expect(cols.contributionFitScore).toBeDefined()
      expect(cols.hasRepro).toBeDefined()
      expect(cols.hasClearDescription).toBeDefined()
    })
  })

  describe('repo_scores table', () => {
    it('has scoring columns', () => {
      const cols = getTableColumns(repoScores)
      expect(cols.stars).toBeDefined()
      expect(cols.contributorCount).toBeDefined()
      expect(cols.contributionOpportunityScore).toBeDefined()
      expect(cols.maintainerActivityScore).toBeDefined()
    })
  })

  describe('ai_drafts table', () => {
    it('has quality gate columns', () => {
      const cols = getTableColumns(aiDrafts)
      expect(cols.qualityRating).toBeDefined()
      expect(cols.qualityAssessment).toBeDefined()
      expect(cols.draftType).toBeDefined()
      expect(cols.status).toBeDefined()
    })
  })
})
