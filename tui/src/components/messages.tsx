import type { ChatMessage } from "../types"
import { theme } from "../theme"
import { For, Show } from "solid-js"

type Props = {
  messages: ChatMessage[]
  streamingText: string
  streaming: boolean
}

const SPINNER = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"]
let spinTick = 0

function spinChar() {
  spinTick = (spinTick + 1) % SPINNER.length
  return SPINNER[spinTick]
}

function RoleBadge(props: { role: "user" | "assistant" }) {
  return (
    <text fg={props.role === "user" ? theme.primary : theme.accent}>
      {props.role === "user" ? "▸ you" : "▸ roc"}
    </text>
  )
}

function MessageBlock(props: { msg: ChatMessage }) {
  return (
    <box flexDirection="column" paddingBottom={1}>
      <box flexDirection="row" gap={1}>
        <RoleBadge role={props.msg.role} />
        <Show when={props.msg.agent}>
          <text fg={theme.textMuted}>[{props.msg.agent}]</text>
        </Show>
        <Show when={props.msg.duration_ms}>
          <text fg={theme.textMuted}>
            {(props.msg.duration_ms! / 1000).toFixed(1)}s
          </text>
        </Show>
      </box>
      <box paddingLeft={2}>
        <text fg={theme.text} wrap>
          {props.msg.content}
        </text>
      </box>
    </box>
  )
}

export function Messages(props: Props) {
  return (
    <scrollbox
      flexGrow={1}
      stickyScroll={true}
      stickyStart="bottom"
      paddingLeft={2}
      paddingRight={2}
    >
      <box height={1} />

      <Show when={props.messages.length === 0 && !props.streaming}>
        <box paddingTop={2} paddingBottom={1}>
          <text fg={theme.textMuted}>Start a conversation…</text>
        </box>
      </Show>

      <For each={props.messages}>
        {(msg) => <MessageBlock msg={msg} />}
      </For>

      <Show when={props.streaming}>
        <box flexDirection="column" paddingBottom={1}>
          <box flexDirection="row" gap={1}>
            <text fg={theme.accent}>▸ roc</text>
            <Show when={!props.streamingText}>
              <text fg={theme.textMuted}>{spinChar()} thinking…</text>
            </Show>
          </box>
          <Show when={props.streamingText}>
            <box paddingLeft={2}>
              <text fg={theme.text} wrap>
                {props.streamingText}
              </text>
            </box>
          </Show>
        </box>
      </Show>
    </scrollbox>
  )
}
