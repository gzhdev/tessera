import { inject, type InjectionKey } from 'vue'

// 链接点击拦截（spec「链接点击被拦截」）：markdown 内的链接不直接导航，
// 交给宿主决定如何处理。宿主经 provide 注入；未注入时降级为 console 警告且阻止跳转。
export type LinkClickHandler = (href: string) => void

export const linkClickHandlerKey: InjectionKey<LinkClickHandler> = Symbol('tessera:link-click')

export function useLinkClickHandler(): LinkClickHandler {
  const handler = inject(linkClickHandlerKey)
  return (
    handler ??
    ((href: string) => {
      // 宿主未接入时的兜底：绝不导航，仅记录（便于开发期发现未接线）
      console.warn(`[tessera] 链接点击未被宿主处理：${href}`)
    })
  )
}
