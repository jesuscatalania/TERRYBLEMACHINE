# Manual QA Checklist

Walk this list before tagging a release or shipping a build for live-testing. Each section ends with edge-cases that the unit/e2e tests can't easily cover (filesystem permissions, real-network failures, multi-window state).

## Boot
- [ ] First start with no projects: empty-state UI guides user to "New project"
- [ ] First start with no API keys: settings panel shows `Add key` for each provider
- [ ] Restart after creating a project: project re-opens at last position

## Website Builder (`/website`)
- [ ] Empty prompt: Generate button disabled
- [ ] Happy-path prompt: 1 result returned, preview pane renders HTML
- [ ] URL analyzer with valid URL: palette + typography extracted
- [ ] URL analyzer with broken URL: error toast, no crash
- [ ] Code editor: Monaco loads, edits propagate to preview
- [ ] Export: HTML + CSS download or saved to disk

## Graphic 2D (`/graphic2d`)
- [ ] Empty prompt: Generate disabled
- [ ] 1 / 6 variants: gallery shows correct count
- [ ] Click variant: editor opens with image
- [ ] Edit (text overlay, crop, flip): saved on canvas
- [ ] Export PNG/JPEG/PDF/GIF: file written to expected path
- [ ] Out-of-budget: warning toast before generation

## Graphic 3D (`/graphic3d`)
- [ ] Depth from image: image upload → depth map renders
- [ ] Text-to-mesh: prompt → mesh appears in 3D viewport
- [ ] Camera controls: rotate / pan / zoom respond to mouse
- [ ] Lighting preset switches: rim / softbox / studio change scene lighting
- [ ] Export glTF / animated GIF: file written

## Video (`/video`)
- [ ] Prompt → storyboard: shots returned with durations
- [ ] Edit a shot: change persists across re-render
- [ ] Render with Remotion: mp4 file written
- [ ] Render with Shotstack: cloud render polls + downloads mp4
- [ ] Render mid-flight + click another module: render continues, navigation works

## Typography (`/typography`)
- [ ] Empty prompt: Generate disabled
- [ ] Generate 6 variants: gallery populates
- [ ] Favorites toggle: heart icon flips, "Show favorites only" filters
- [ ] Favorites filter resets after re-Generate
- [ ] Click variant: editor enables Vectorize
- [ ] Vectorize: SVG renders in canvas
- [ ] Add text: input + button works, Textbox appears centered
- [ ] Slider edits (font / color / size / kerning): apply to selected Textbox in real time
- [ ] Export brand kit: dialog opens, accepts brand name + colors + font + destination
- [ ] Export with non-existent destination: clear error toast
- [ ] Export with valid destination: ZIP written, contains 12 files (svg + 8 sizes + bw + inverted + style-guide.html)

## Cross-cutting
- [ ] Sidebar module switch preserves in-flight state
- [ ] Browser back / forward: history works
- [ ] Window resize: layout reflows, no cropped content
- [ ] Cmd+Z / Cmd+Shift+Z: undo/redo on canvas-bearing pages
- [ ] No console errors on any happy-path flow
- [ ] No native panic in any flow (check Console.app for `terryblemachine` crash logs)
