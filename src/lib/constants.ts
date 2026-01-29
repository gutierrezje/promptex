export const GITHUB_OAUTH_URL = 'https://github.com/login/oauth/authorize'
export const GITHUB_TOKEN_URL = 'https://github.com/login/oauth/access_token'
export const GITHUB_API_URL = 'https://api.github.com'

export const GITHUB_SCOPES = 'repo read:user'

export const SYNC_STALE_THRESHOLD_MS = 5 * 60 * 1000 // 5 minutes
export const CLIENT_POLL_INTERVAL_MS = 60 * 1000 // 60 seconds
export const INSIGHTS_INTERVAL_HOURS = 4

export const EVENT_TYPES = [
  'issue_opened',
  'issue_closed',
  'issue_comment',
  'push',
  'pr_opened',
  'pr_merged',
  'pr_closed',
  'release',
  'label_change',
  'assignment',
] as const

export type EventType = (typeof EVENT_TYPES)[number]
