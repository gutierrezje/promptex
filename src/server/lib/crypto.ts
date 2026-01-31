import { createCipheriv, createDecipheriv, randomBytes, scryptSync } from 'crypto'
import { env } from './env'

const ALGORITHM = 'aes-256-gcm'
const KEY = scryptSync(env.SESSION_SECRET, 'issuance-salt', 32)

export function encrypt(plaintext: string): string {
  const iv = randomBytes(12)
  const cipher = createCipheriv(ALGORITHM, KEY, iv)
  
  const ciphertext = cipher.update(plaintext, 'utf8', 'base64') + cipher.final('base64')
  const authTag = cipher.getAuthTag()

  return `${iv.toString('base64')}:${authTag.toString('base64')}:${ciphertext}`
}

export function decrypt(encrypted: string): string {
  const [ivB64, authTagB64, ciphertext] = encrypted.split(':')
  const iv = Buffer.from(ivB64, 'base64')
  const authTag = Buffer.from(authTagB64, 'base64')

  const decipher = createDecipheriv(ALGORITHM, KEY, iv)
  decipher.setAuthTag(authTag)

  return decipher.update(ciphertext, 'base64', 'utf8') + decipher.final('utf8')
}
