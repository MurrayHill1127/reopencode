import { theme } from "../theme"
import { Show, onMount } from "solid-js"
import { useKeyboard } from "@opentui/solid"
import type { TextareaRenderable } from "@opentui/core"

type Props = {
  streaming: boolean
  onSubmit: (text: string) => void
}

export function Prompt(props: Props) {
  let textarea: TextareaRenderable

  useKeyboard((evt) => {
    if (props.streaming) {
      evt.preventDefault()
      return
    }
    if (evt.name === "return" && !evt.shift) {
      evt.preventDefault()
      const text = textarea?.plainText?.trim() ?? ""
      if (text) {
        props.onSubmit(text)
        textarea?.clear()
      }
    }
  })

  return (
    <box
      flexDirection="column"
      borderStyle="round"
      borderColor={props.streaming ? theme.accent : theme.border}
      paddingLeft={1}
      paddingRight={1}
      height={5}
    >
      <box flexDirection="row" alignItems="center" gap={1}>
        <Show when={props.streaming}>
          <text fg={theme.accent}>⠿</text>
        </Show>
        <Show when={!props.streaming}>
          <text fg={theme.primary}>›</text>
        </Show>
        <textarea
          ref={(r: TextareaRenderable) => { textarea = r }}
          placeholder="Type a message…"
          placeholderColor={theme.textMuted}
          textColor={theme.text}
          focusedTextColor={theme.text}
          minHeight={1}
          maxHeight={4}
          flexGrow={1}
        />
      </box>
    </box>
  )
}
