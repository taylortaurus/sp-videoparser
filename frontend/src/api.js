// 后端 API 客户端。统一响应 {code,msg,data}，code=200 视为成功，否则抛出 msg。

async function getData(url) {
  let res
  try {
    res = await fetch(url, { headers: { Accept: 'application/json' } })
  } catch {
    // fetch 仅在网络层失败时 reject（断网、服务不可达、请求被拦截等），避免把原始英文错误漏给用户
    throw new Error('网络连接失败，请检查网络或稍后重试')
  }
  let body
  try {
    body = await res.json()
  } catch {
    throw new Error(`服务异常 (HTTP ${res.status})`)
  }
  if (body.code !== 200) {
    throw new Error(body.msg || `请求失败 (HTTP ${res.status})`)
  }
  return body.data
}

/** 按分享链接或含链接的文案解析 */
export function parseShareUrl(text) {
  return getData(`/api/video/share/url/parse?url=${encodeURIComponent(text)}`)
}

/** 按平台标识 + 视频 ID 解析 */
export function parseById(source, videoId) {
  return getData(
    `/api/video/id/parse?source=${encodeURIComponent(source)}&video_id=${encodeURIComponent(videoId)}`
  )
}

/** 全部受支持平台 */
export function getPlatforms() {
  return getData('/api/platforms')
}

/** 视频代理地址（支持 Range，可用于 <video> 拖动播放与下载） */
export function proxyVideo(url) {
  return `/api/proxy/video?url=${encodeURIComponent(url)}`
}

/** 图片代理地址 */
export function proxyImage(url) {
  return `/api/proxy/image?url=${encodeURIComponent(url)}`
}
