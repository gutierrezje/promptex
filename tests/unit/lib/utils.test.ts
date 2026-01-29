import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { cn, formatRelativeTime } from '@/lib/utils'

describe('cn', () => {
  it('merges class names', () => {
    expect(cn('foo', 'bar')).toBe('foo bar')
  })

  it('handles conditional classes', () => {
    expect(cn('base', false && 'hidden', 'visible')).toBe('base visible')
  })

  it('deduplicates tailwind classes', () => {
    expect(cn('px-2 py-1', 'px-4')).toBe('py-1 px-4')
  })
})

describe('formatRelativeTime', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2025-06-15T12:00:00Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('returns "just now" for times less than 60s ago', () => {
    const date = new Date('2025-06-15T11:59:30Z')
    expect(formatRelativeTime(date)).toBe('just now')
  })

  it('returns minutes for times less than 1h ago', () => {
    const date = new Date('2025-06-15T11:45:00Z')
    expect(formatRelativeTime(date)).toBe('15m ago')
  })

  it('returns hours for times less than 24h ago', () => {
    const date = new Date('2025-06-15T06:00:00Z')
    expect(formatRelativeTime(date)).toBe('6h ago')
  })

  it('returns days for times less than 30d ago', () => {
    const date = new Date('2025-06-10T12:00:00Z')
    expect(formatRelativeTime(date)).toBe('5d ago')
  })

  it('returns a formatted date for times over 30d ago', () => {
    const date = new Date('2025-01-01T12:00:00Z')
    const result = formatRelativeTime(date)
    // Should be a locale date string, not "Xd ago"
    expect(result).not.toContain('ago')
  })
})
