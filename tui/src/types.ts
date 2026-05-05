export type Session = {
  id: string
  title: string
  created_at: string
  updated_at: string
  status: string
  message_count: number
}

export type ChatMessage = {
  role: "user" | "assistant"
  content: string
  agent?: string
  model?: string
  duration_ms?: number
}

export type AppState = {
  session: Session | null
  messages: ChatMessage[]
  streaming: boolean
  streamingText: string
  status: string
  directory: string
}
