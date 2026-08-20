// 生成占位应用图标：32x32 经典 DIB 格式 ICO（RC.EXE 不认 PNG-in-ICO）。
// 仅脚手架用；正式图标由设计资源替换。
import { deflateSync } from 'node:zlib'
import { writeFileSync, mkdirSync } from 'node:fs'

const W = 32
const H = 32

// 深灰蓝底，右下角亮色小方块作辨识
const px = (x, y) => {
  const accent = x >= 20 && y >= 20
  return accent ? [0x7a, 0x9e, 0xd9, 0xff] : [0x2b, 0x2f, 0x3a, 0xff]
}

// ---- ICO：DIB（BITMAPINFOHEADER + BGRA 自底向上）+ 1bpp AND 掩码 ----
const dibHeader = Buffer.alloc(40)
dibHeader.writeUInt32LE(40, 0)      // biSize
dibHeader.writeInt32LE(W, 4)        // biWidth
dibHeader.writeInt32LE(H * 2, 8)    // biHeight：XOR + AND 两倍高
dibHeader.writeUInt16LE(1, 12)      // biPlanes
dibHeader.writeUInt16LE(32, 14)     // biBitCount
// 其余字段（压缩/尺寸/分辨率/调色板）全 0

const xor = Buffer.alloc(H * W * 4)
for (let y = 0; y < H; y++) {
  const srcY = H - 1 - y // 自底向上
  for (let x = 0; x < W; x++) {
    const [r, g, b, a] = px(x, srcY)
    const o = (y * W + x) * 4
    xor[o] = b; xor[o + 1] = g; xor[o + 2] = r; xor[o + 3] = a
  }
}
const andMask = Buffer.alloc(H * 4) // 32px @1bpp = 4 字节/行，全 0（不透明）

const dib = Buffer.concat([dibHeader, xor, andMask])

const header = Buffer.alloc(6)
header.writeUInt16LE(0, 0); header.writeUInt16LE(1, 2); header.writeUInt16LE(1, 4)
const entry = Buffer.alloc(16)
entry[0] = W; entry[1] = H
entry.writeUInt16LE(1, 4)        // planes
entry.writeUInt16LE(32, 6)       // bpp
entry.writeUInt32LE(dib.length, 8)
entry.writeUInt32LE(6 + 16, 12)

mkdirSync('src-tauri/icons', { recursive: true })
writeFileSync('src-tauri/icons/icon.ico', Buffer.concat([header, entry, dib]))

// ---- 附带一枚 PNG（Linux 捆绑等场景备用） ----
const raw = Buffer.alloc(H * (1 + W * 4))
for (let y = 0; y < H; y++) {
  const row = y * (1 + W * 4)
  raw[row] = 0
  for (let x = 0; x < W; x++) {
    const [r, g, b, a] = px(x, y)
    const o = row + 1 + x * 4
    raw[o] = r; raw[o + 1] = g; raw[o + 2] = b; raw[o + 3] = a
  }
}
const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
  return c >>> 0
})
const crc32 = (buf) => {
  let c = 0xffffffff
  for (const byte of buf) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8)
  return (c ^ 0xffffffff) >>> 0
}
const chunk = (type, data) => {
  const len = Buffer.alloc(4); len.writeUInt32BE(data.length)
  const body = Buffer.concat([Buffer.from(type), data])
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(body))
  return Buffer.concat([len, body, crc])
}
const ihdr = Buffer.alloc(13)
ihdr.writeUInt32BE(W, 0); ihdr.writeUInt32BE(H, 4)
ihdr[8] = 8; ihdr[9] = 6
writeFileSync('src-tauri/icons/icon.png', Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(raw)),
  chunk('IEND', Buffer.alloc(0)),
]))
console.log('图标已生成（DIB ICO + PNG）')
