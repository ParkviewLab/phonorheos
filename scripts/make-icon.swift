// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Draws phonorheos's icon, the mirror of graptostream's (sound turning into written
// lines): written lines turning into sound. Writes brand/app-icon/icon.png (1024 px),
// from which `cargo tauri icon` makes the app's icons.
// Run: swift scripts/make-icon.swift && cargo tauri icon brand/app-icon/icon.png -o crates/phonorheos-app/icons

import AppKit

func draw(size s: CGFloat) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(s), pixelsHigh: Int(s), bitsPerSample: 8,
                               samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
                               bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let inset = s * 0.1
    let tile = NSRect(x: inset, y: inset, width: s - 2 * inset, height: s - 2 * inset)
    let shape = NSBezierPath(roundedRect: tile, xRadius: s * 0.18, yRadius: s * 0.18)
    NSGradient(colors: [NSColor(srgbRed: 0.10, green: 0.38, blue: 0.45, alpha: 1),
                        NSColor(srgbRed: 0.16, green: 0.22, blue: 0.52, alpha: 1)])!.draw(in: shape, angle: -90)

    // Left: lines of writing. Right: the same rhythm as a waveform.
    let mid = s * 0.5
    NSColor(white: 1, alpha: 0.95).setFill()
    let lines: [CGFloat] = [0.20, 0.15, 0.18, 0.10]
    let lineHeight = s * 0.045
    for (i, w) in lines.enumerated() {
        let y = mid + s * 0.15 - CGFloat(i) * lineHeight * 2.2
        let rect = NSRect(x: tile.minX + s * 0.09, y: y - lineHeight / 2, width: s * w, height: lineHeight)
        NSBezierPath(roundedRect: rect, xRadius: lineHeight / 2, yRadius: lineHeight / 2).fill()
    }
    let bars: [CGFloat] = [0.12, 0.28, 0.18, 0.34, 0.22, 0.10]
    let barWidth = s * 0.035
    for (i, h) in bars.enumerated() {
        let x = tile.maxX - s * 0.09 - barWidth - CGFloat(bars.count - 1 - i) * barWidth * 1.9
        let rect = NSRect(x: x, y: mid - s * h / 2, width: barWidth, height: s * h)
        NSBezierPath(roundedRect: rect, xRadius: barWidth / 2, yRadius: barWidth / 2).fill()
    }
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

let out = URL(fileURLWithPath: "brand/app-icon/icon.png")
try FileManager.default.createDirectory(at: out.deletingLastPathComponent(), withIntermediateDirectories: true)
try draw(size: 1024).representation(using: .png, properties: [:])!.write(to: out)
print("wrote \(out.path)")
