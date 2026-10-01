import CoreGraphics

/// Procedural 12MP test image. Keeps the repo text-only (no binary assets)
/// and simulates a real camera photo's texture budget (4032×3024 = 12.2MP).
enum TestImage {
    static let width = 4032
    static let height = 3024

    static func make() -> CGImage? {
        guard let ctx = CGContext(
            data: nil, width: width, height: height,
            bitsPerComponent: 8, bytesPerRow: width * 4,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
        else { return nil }

        // Gradient backdrop (CG origin is bottom-left).
        let grad = CGGradient(
            colorsSpace: CGColorSpaceCreateDeviceRGB(),
            colors: [
                CGColor(red: 0.95, green: 0.55, blue: 0.30, alpha: 1),
                CGColor(red: 0.25, green: 0.40, blue: 0.80, alpha: 1),
            ] as CFArray, locations: [0, 1])!
        ctx.drawLinearGradient(grad, start: CGPoint(x: 0, y: 0),
                               end: CGPoint(x: CGFloat(width), y: CGFloat(height)), options: [])

        // Color bars — make slider effects visually obvious.
        let bars: [CGColor] = [
            CGColor(red: 1, green: 1, blue: 1, alpha: 1),
            CGColor(red: 1, green: 1, blue: 0, alpha: 1),
            CGColor(red: 0, green: 1, blue: 1, alpha: 1),
            CGColor(red: 0, green: 1, blue: 0, alpha: 1),
            CGColor(red: 1, green: 0, blue: 1, alpha: 1),
            CGColor(red: 1, green: 0, blue: 0, alpha: 1),
            CGColor(red: 0, green: 0, blue: 1, alpha: 1),
            CGColor(red: 0, green: 0, blue: 0, alpha: 1),
        ]
        let barW = CGFloat(width) / CGFloat(bars.count)
        for (i, c) in bars.enumerated() {
            ctx.setFillColor(c)
            ctx.fill(CGRect(x: CGFloat(i) * barW, y: CGFloat(height) * 0.38,
                            width: barW, height: CGFloat(height) * 0.20))
        }

        // High-frequency rings — expose sharpen/blur differences.
        let center = CGPoint(x: CGFloat(width) / 2, y: CGFloat(height) / 2)
        for (i, r) in stride(from: 120.0, through: 900.0, by: 120.0).enumerated() {
            ctx.setStrokeColor(CGColor(red: 1, green: 0.9, blue: 0.2, alpha: i.isMultiple(of: 2) ? 1 : 0.4))
            ctx.setLineWidth(14)
            ctx.strokeEllipse(in: CGRect(x: center.x - r, y: center.y - r, width: r * 2, height: r * 2))
        }
        return ctx.makeImage()
    }
}
