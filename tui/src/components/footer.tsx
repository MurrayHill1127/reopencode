import { theme } from "../theme"

type Props = {
  status: string
  directory: string
  streaming: boolean
}

export function Footer(props: Props) {
  const dir = () => {
    const d = props.directory
    if (d.length <= 32) return d
    const parts = d.split("/")
    return "…/" + parts.slice(-2).join("/")
  }

  return (
    <box
      flexDirection="row"
      height={1}
      paddingLeft={1}
      paddingRight={1}
      backgroundColor={theme.backgroundPanel}
      alignItems="center"
      justifyContent="space-between"
    >
      <box flexDirection="row" gap={2}>
        <text fg={theme.textMuted}>{dir()}</text>
        <text fg={props.streaming ? theme.warning : theme.textMuted}>
          {props.streaming ? "streaming" : props.status}
        </text>
      </box>
      <text fg={theme.textMuted}>
        <span style={{ fg: theme.primary }}>?</span> help
      </text>
    </box>
  )
}
