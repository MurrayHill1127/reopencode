import { createCliRenderer } from "@opentui/core"
import { render } from "@opentui/solid"
import { App } from "./src/app"

const renderer = await createCliRenderer({
  exitOnCtrlC: false,
  targetFps: 30,
})

renderer.start()

await render(() => <App />, renderer)
