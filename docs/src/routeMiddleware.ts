import { defineRouteMiddleware } from '@astrojs/starlight/route-data'
import { extractDescriptionFromMarkdown } from './lib/extract-description.mjs'

// Starlight 的 head 去重保留先出现的条目，所以要替换已有 meta 而不是 push 新的
function upsertMeta(head, key, content) {
  const [attr, value] = key
  const existing = head.find((entry) => entry.tag === 'meta' && entry.attrs?.[attr] === value)
  if (existing) {
    existing.attrs.content = content
  } else {
    head.push({ tag: 'meta', attrs: { [attr]: value, content } })
  }
}

export const onRequest = defineRouteMiddleware((context) => {
  const { entry, head, locale } = context.locals.starlightRoute

  // 404 页不进索引，延续旧文档站的 noindex 行为
  if (entry.id === '404' || entry.id === 'en/404') {
    head.push({ tag: 'meta', attrs: { name: 'robots', content: 'noindex, nofollow' } })
    return
  }

  // frontmatter 没写 description 时，从 markdown 正文推断
  if (!entry.data.description) {
    const inferred = extractDescriptionFromMarkdown(entry.body ?? '')
    if (inferred) {
      upsertMeta(head, ['name', 'description'], inferred)
      upsertMeta(head, ['property', 'og:description'], inferred)
      upsertMeta(head, ['name', 'twitter:description'], inferred)
    }
  }

  // Starlight 只生成 og:locale，补上 alternate 保持与旧站一致
  const isEn = locale === 'en'
  head.push({
    tag: 'meta',
    attrs: { property: 'og:locale:alternate', content: isEn ? 'zh_CN' : 'en_US' }
  })
})
