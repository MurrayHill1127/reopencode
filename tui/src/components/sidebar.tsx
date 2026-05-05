import type { Session } from "../types"
import { theme } from "../theme"
import { Show } from "solid-js"

type Props = {
  session: Session | null
  status: string
}

export function Sidebar(props: Props) {
  return (
    <box
      backgroundColor={theme.backgroundPanel}
      width={42}
      height="100%"
      paddingTop={1}
      paddingBottom={1}
      paddingLeft={2}
      paddingRight={2}
      flexDirection="column"
      justifyContent="space-between"
    >
      {/* Top: session title + keybind hints */}
      <box flexDirection="column" gap={1} flexGrow={1}>
        <box paddingRight={1}>
          <text fg={theme.text}>
            <b>{props.session?.title ?? "New Session"}</b>
          </text>
          <Show when={props.session}>
            <text fg={theme.textMuted}>
              {props.session!.id.slice(0, 8)}
            </text>
          </Show>
        </box>

        <box flexDirection="column" gap={0}>
          <text fg={theme.textMuted}>
            <span style={{ fg: theme.primary }}>Enter </span>
            send
          </text>
          <text fg={theme.textMuted}>
            <span style={{ fg: theme.primary }}>Ctrl+p </span>
            sessions
          </text>
          <text fg={theme.textMuted}>
            <span style={{ fg: theme.primary }}>Ctrl+b </span>
            sidebar
          </text>
          <text fg={theme.textMuted}>
            <span style={{ fg: theme.primary }}>Ctrl+c </span>
            quit
          </text>
        </box>
      </box>

      {/* Bottom: branding */}
      <text fg={theme.textMuted}>
        <span style={{ fg: theme.success }}>●</span>{" "}
        <b>Re</b>
        <span style={{ fg: theme.text }}>
          <b>Open</b>
        </span>
        <span style={{ fg: theme.primary }}>Code</span>
      </text>
    </box>
  )
}
