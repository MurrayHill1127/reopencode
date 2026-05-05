import type { Session } from "../types"
import { theme } from "../theme"
import { For, Show } from "solid-js"
import { useKeyboard } from "@opentui/solid"

type Props = {
  sessions: Session[]
  onSelect: (s: Session) => void
  onClose: () => void
  onNew: () => void
}

export function SessionPicker(props: Props) {
  useKeyboard((evt) => {
    if (evt.name === "escape") {
      evt.preventDefault()
      props.onClose()
    }
    if (evt.name === "n") {
      evt.preventDefault()
      props.onNew()
    }
  })

  return (
    <box
      position="absolute"
      top={2}
      left={4}
      width={60}
      height={20}
      flexDirection="column"
      backgroundColor={theme.backgroundPanel}
      borderStyle="round"
      borderColor={theme.borderActive}
    >
      <box
        paddingLeft={2}
        paddingRight={2}
        paddingTop={1}
        paddingBottom={1}
        flexDirection="row"
        justifyContent="space-between"
      >
        <text fg={theme.text}><b>Sessions</b></text>
        <text fg={theme.textMuted}>Esc close  ·  n new</text>
      </box>

      <scrollbox
        flexGrow={1}
        paddingLeft={1}
        paddingRight={1}
      >
        <Show when={props.sessions.length === 0}>
          <text fg={theme.textMuted} paddingLeft={1}>No sessions yet</text>
        </Show>
        <For each={props.sessions}>
          {(s) => (
            <box
              flexDirection="row"
              justifyContent="space-between"
              paddingLeft={1}
              paddingRight={1}
              height={1}
              onMouseDown={() => props.onSelect(s)}
            >
              <text fg={theme.text}>{s.title}</text>
              <text fg={theme.textMuted}>{s.id.slice(0, 8)}</text>
            </box>
          )}
        </For>
      </scrollbox>

      <box paddingLeft={2} paddingBottom={1}>
        <text fg={theme.success} onMouseDown={props.onNew}>+ New session</text>
      </box>
    </box>
  )
}
