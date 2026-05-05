import { createSignal, onMount, onCleanup, Show } from "solid-js"
import { useKeyboard } from "@opentui/solid"
import type { AppState, ChatMessage, Session } from "./types"
import { theme } from "./theme"
import { createSession, listSessions, streamMessage } from "./api"
import { Sidebar } from "./components/sidebar"
import { Messages } from "./components/messages"
import { Prompt } from "./components/prompt"
import { Footer } from "./components/footer"
import { SessionPicker } from "./components/session_picker"

export function App() {
  const [session, setSession] = createSignal<Session | null>(null)
  const [messages, setMessages] = createSignal<ChatMessage[]>([])
  const [streaming, setStreaming] = createSignal(false)
  const [streamingText, setStreamingText] = createSignal("")
  const [status, setStatus] = createSignal("ready")
  const [sidebarOpen, setSidebarOpen] = createSignal(true)
  const [pickerOpen, setPickerOpen] = createSignal(false)
  const [sessions, setSessions] = createSignal<Session[]>([])
  const directory = process.env["ROC_DIR"] ?? process.cwd()

  let abortController: AbortController | null = null

  async function initSession() {
    try {
      const existing = await listSessions()
      setSessions(existing)
      if (existing.length > 0) {
        setSession(existing[0])
      } else {
        const s = await createSession("ROC Session")
        setSessions([s])
        setSession(s)
      }
    } catch (e) {
      setStatus(`error: ${e}`)
    }
  }

  async function openPicker() {
    try {
      const all = await listSessions()
      setSessions(all)
    } catch {}
    setPickerOpen(true)
  }

  function selectSession(s: Session) {
    setSession(s)
    setMessages([])
    setStatus("ready")
    setPickerOpen(false)
  }

  async function newSession() {
    try {
      const s = await createSession("ROC Session")
      setSessions((prev) => [s, ...prev])
      setSession(s)
      setMessages([])
      setStatus("ready")
    } catch (e) {
      setStatus(`error: ${e}`)
    }
    setPickerOpen(false)
  }

  function send(content: string) {
    const s = session()
    if (!content || streaming() || !s) return

    const userMsg: ChatMessage = { role: "user", content }
    setMessages((prev) => [...prev, userMsg])
    setStreaming(true)
    setStreamingText("")
    setStatus("streaming")

    let accumulated = ""
    const start = Date.now()

    abortController = streamMessage(s.id, content, {
      onChunk(chunk) {
        accumulated += chunk
        setStreamingText(accumulated)
      },
      onDone() {
        const duration_ms = Date.now() - start
        setMessages((prev) => [
          ...prev,
          { role: "assistant", content: accumulated, duration_ms },
        ])
        setStreaming(false)
        setStreamingText("")
        setStatus("ready")
        abortController = null
      },
      onError(err) {
        setStatus(`error: ${err}`)
        setStreaming(false)
        setStreamingText("")
        abortController = null
      },
    })
  }

  useKeyboard((evt) => {
    if (evt.ctrl && evt.name === "c") {
      if (abortController) {
        abortController.abort()
        abortController = null
        setStreaming(false)
        setStreamingText("")
        setStatus("cancelled")
        evt.preventDefault()
        return
      }
      process.exit(0)
    }
    if (evt.ctrl && evt.name === "b") {
      setSidebarOpen((v) => !v)
      evt.preventDefault()
      return
    }
    if (evt.ctrl && evt.name === "p") {
      if (pickerOpen()) setPickerOpen(false)
      else openPicker()
      evt.preventDefault()
      return
    }
    if (evt.name === "escape" && pickerOpen()) {
      setPickerOpen(false)
      evt.preventDefault()
      return
    }
  })

  onMount(() => {
    initSession()
  })

  onCleanup(() => {
    abortController?.abort()
  })

  return (
    <box
      width="100%"
      height="100%"
      flexDirection="row"
      backgroundColor={theme.background}
    >
      <Show when={sidebarOpen()}>
        <Sidebar session={session()} status={status()} />
      </Show>

      <box flexDirection="column" flexGrow={1}>
        <Messages
          messages={messages()}
          streamingText={streamingText()}
          streaming={streaming()}
        />
        <Prompt
          streaming={streaming()}
          onSubmit={send}
        />
        <Footer
          status={status()}
          directory={directory}
          streaming={streaming()}
        />
      </box>

      <Show when={pickerOpen()}>
        <SessionPicker
          sessions={sessions()}
          onSelect={selectSession}
          onClose={() => setPickerOpen(false)}
          onNew={newSession}
        />
      </Show>
    </box>
  )
}
