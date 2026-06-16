import { onMounted, onUnmounted } from 'vue'
import { usePlaybackStore } from '@/stores/playback'

export function useKeyboardShortcuts() {
  const store = usePlaybackStore()

  function isInputFocused(target: EventTarget | null): boolean {
    if (!target || !(target instanceof HTMLElement)) return false
    const tag = target.tagName.toLowerCase()
    if (tag === 'input' || tag === 'textarea' || tag === 'select') return true
    if ((target as HTMLElement).isContentEditable) return true
    return false
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (isInputFocused(e.target)) return

    const ctrl = e.ctrlKey || e.metaKey

    switch (e.code) {
      case 'Space':
        e.preventDefault()
        store.togglePlay()
        break
      case 'ArrowLeft':
        e.preventDefault()
        if (ctrl) {
          store.prev()
        } else {
          store.seekTo(Math.max(0, store.position - 5))
        }
        break
      case 'ArrowRight':
        e.preventDefault()
        if (ctrl) {
          store.next()
        } else {
          store.seekTo(Math.min(store.duration, store.position + 5))
        }
        break
      case 'ArrowUp':
        e.preventDefault()
        store.setVolume(Math.min(1, store.volume + 0.05))
        break
      case 'ArrowDown':
        e.preventDefault()
        store.setVolume(Math.max(0, store.volume - 0.05))
        break
    }
  }

  onMounted(() => {
    window.addEventListener('keydown', handleKeyDown)
  })

  onUnmounted(() => {
    window.removeEventListener('keydown', handleKeyDown)
  })
}
