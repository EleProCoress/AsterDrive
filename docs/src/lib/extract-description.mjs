// 文档页 description 推断逻辑，不包含文件 IO 和缓存包装。
// 输入 markdown 原文，输出适合 <meta name="description"> 的摘要文本。

const PAGE_DESCRIPTION_LIMIT = 160
const MIN_USEFUL_DESCRIPTION_LENGTH = 24

function stripFrontmatter(source) {
  const normalizedSource = source.replace(/^﻿/, '')
  const match = normalizedSource.match(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/)
  return match ? normalizedSource.slice(match[0].length) : normalizedSource
}

function normalizeInlineMarkdown(text) {
  return text
    .replace(/!\[([^\]]*)\]\(([^)]+)\)/g, '$1')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '$1')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/[*_]/g, '')
    .replace(/<[^>]+>/g, '')
    .replace(/\s+/g, ' ')
    .replace(/\s+([，。！？；：,.!?;:])/g, '$1')
    .trim()
}

function truncateDescription(text) {
  if (text.length <= PAGE_DESCRIPTION_LIMIT) {
    return text
  }

  const sliced = text.slice(0, PAGE_DESCRIPTION_LIMIT).replace(/[\s，。！？；：,.!?;:]+$/u, '')
  return `${sliced}…`
}

export function extractDescriptionFromMarkdown(source) {
  const lines = stripFrontmatter(source).split(/\r?\n/)
  let shortFallback = ''

  for (let index = 0; index < lines.length; ) {
    const line = lines[index].trim()

    if (!line) {
      index++
      continue
    }

    if (line.startsWith('#')) {
      index++
      continue
    }

    if (/^:::\s*/.test(line)) {
      const customBlockLines = []
      index++
      while (index < lines.length && !/^\s*:::\s*$/.test(lines[index].trim())) {
        customBlockLines.push(lines[index])
        index++
      }
      if (index < lines.length) {
        index++
      }

      const customBlockDescription = extractDescriptionFromMarkdown(customBlockLines.join('\n'))
      if (customBlockDescription.length >= MIN_USEFUL_DESCRIPTION_LENGTH) {
        return customBlockDescription
      }
      if (customBlockDescription && !shortFallback) {
        shortFallback = customBlockDescription
      }

      continue
    }

    if (/^```/.test(line) || /^~~~/.test(line)) {
      const fence = line.startsWith('```') ? '```' : '~~~'
      index++
      while (index < lines.length && !lines[index].trim().startsWith(fence)) {
        index++
      }
      if (index < lines.length) {
        index++
      }
      continue
    }

    if (/^[>*+\-|]\s/.test(line) || /^\|/.test(line)) {
      index++
      while (index < lines.length && lines[index].trim()) {
        index++
      }
      continue
    }

    const paragraphLines = [line]
    index++

    while (index < lines.length) {
      const nextLine = lines[index].trim()
      if (!nextLine) {
        break
      }
      if (
        nextLine.startsWith('#') ||
        /^:::\s*/.test(nextLine) ||
        /^```/.test(nextLine) ||
        /^~~~/.test(nextLine) ||
        /^[>*+\-|]\s/.test(nextLine) ||
        /^\|/.test(nextLine)
      ) {
        break
      }
      paragraphLines.push(nextLine)
      index++
    }

    const paragraph = normalizeInlineMarkdown(paragraphLines.join(' '))
    if (!paragraph) {
      continue
    }

    if (paragraph.length >= MIN_USEFUL_DESCRIPTION_LENGTH) {
      return truncateDescription(paragraph)
    }

    if (!shortFallback) {
      shortFallback = paragraph
    }
  }

  return shortFallback ? truncateDescription(shortFallback) : ''
}
