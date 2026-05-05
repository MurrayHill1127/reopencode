import type { Session } from "./types"

const BASE = "http://127.0.0.1:4096"

export async function createSession(title?: string): Promise<Session> {
  const res = await fetch(`${BASE}/session`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ title: title ?? "ROC Session" }),
  })
  if (!res.ok) throw new Error(`createSession HTTP ${res.status}`)
  return res.json() as Promise<Session>
}

export async function listSessions(): Promise<Session[]> {
  const res = await fetch(`${BASE}/session`)
  if (!res.ok) throw new Error(`listSessions HTTP ${res.status}`)
  return res.json() as Promise<Session[]>
}

export type StreamHandler = {
  onChunk: (text: string) => void
  onDone: () => void
  onError: (err: string) => void
}

export function streamMessage(
  sessionId: string,
  content: string,
  handler: StreamHandler,
): AbortController {
  const ac = new AbortController()

  ;(async () => {
    try {
      const res = await fetch(`${BASE}/session/${sessionId}/stream`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ content }),
        signal: ac.signal,
      })

      if (!res.ok) {
        handler.onError(`HTTP ${res.status}`)
        return
      }

      if (!res.body) {
        handler.onError("No response body")
        return
      }

      const reader = res.body.getReader()
      const decoder = new TextDecoder()
      let buf = ""

      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        buf += decoder.decode(value, { stream: true })

        // SSE events separated by \n\n
        while (buf.includes("\n\n")) {
          const idx = buf.indexOf("\n\n")
          const event = buf.slice(0, idx)
          buf = buf.slice(idx + 2)

          for (const line of event.split("\n")) {
            if (line.startsWith("data: ")) {
              handler.onChunk(line.slice(6))
            }
          }
        }
      }

      handler.onDone()
    } catch (e: unknown) {
      if (e instanceof Error && e.name === "AbortError") return
      handler.onError(String(e))
    }
  })()

  return ac
}
