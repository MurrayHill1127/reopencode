import { RGBA } from "@opentui/core"

// opencode palette — dark mode
export const theme = {
  background:        RGBA.fromHex("#0a0a0a"),
  backgroundPanel:   RGBA.fromHex("#141414"),
  backgroundElement: RGBA.fromHex("#1e1e1e"),
  border:            RGBA.fromHex("#484848"),
  borderActive:      RGBA.fromHex("#606060"),
  borderSubtle:      RGBA.fromHex("#3c3c3c"),

  text:       RGBA.fromHex("#eeeeee"),
  textMuted:  RGBA.fromHex("#808080"),

  primary:  RGBA.fromHex("#fab283"),  // warm orange
  accent:   RGBA.fromHex("#9d7cd8"),  // purple
  success:  RGBA.fromHex("#7fd88f"),  // green
  warning:  RGBA.fromHex("#f5a742"),  // orange
  error:    RGBA.fromHex("#e06c75"),  // red
  info:     RGBA.fromHex("#56b6c2"),  // cyan
}
