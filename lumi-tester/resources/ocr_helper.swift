#!/usr/bin/swift
// macOS Vision OCR Helper with Image Resizing for Performance
// Usage: swift ocr_helper.swift <image_path>
// Output: TSV format - text, x, y, width, height

import AppKit
import Foundation
import Vision

let MAX_WIDTH: CGFloat = 1080  // Resize if larger than this

guard CommandLine.arguments.count > 1 else {
  fputs("Usage: ocr_helper.swift <image_path>\n", stderr)
  exit(1)
}

let imagePath = CommandLine.arguments[1]
guard let image = NSImage(contentsOfFile: imagePath) else {
  fputs("Error: Cannot load image: \(imagePath)\n", stderr)
  exit(1)
}

// Get original size
let originalSize = image.size
var processedImage = image
var scaleFactor: CGFloat = 1.0

// Resize if image is too large
if originalSize.width > MAX_WIDTH {
  scaleFactor = originalSize.width / MAX_WIDTH
  let newHeight = originalSize.height / scaleFactor
  let newSize = NSSize(width: MAX_WIDTH, height: newHeight)

  let resizedImage = NSImage(size: newSize)
  resizedImage.lockFocus()
  image.draw(
    in: NSRect(origin: .zero, size: newSize),
    from: NSRect(origin: .zero, size: originalSize),
    operation: .copy,
    fraction: 1.0)
  resizedImage.unlockFocus()
  processedImage = resizedImage
}

guard let cgImage = processedImage.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
  fputs("Error: Cannot convert to CGImage\n", stderr)
  exit(1)
}

let imageWidth = CGFloat(cgImage.width)
let imageHeight = CGFloat(cgImage.height)

let request = VNRecognizeTextRequest { request, error in
  if let error = error {
    fputs("Error: \(error.localizedDescription)\n", stderr)
    exit(1)
  }

  guard let observations = request.results as? [VNRecognizedTextObservation] else {
    return
  }

  for observation in observations {
    guard let topCandidate = observation.topCandidates(1).first else { continue }

    let text = topCandidate.string
    let boundingBox = observation.boundingBox

    // Convert normalized coordinates to pixel coordinates (scaled back to original)
    let x = Int(boundingBox.origin.x * imageWidth * scaleFactor)
    let y = Int((1 - boundingBox.origin.y - boundingBox.height) * imageHeight * scaleFactor)
    let width = Int(boundingBox.width * imageWidth * scaleFactor)
    let height = Int(boundingBox.height * imageHeight * scaleFactor)

    // Output TSV: text, x, y, width, height
    print("\(text)\t\(x)\t\(y)\t\(width)\t\(height)")
  }
}

// Configure for accuracy with Vietnamese support
request.recognitionLevel = .accurate
request.usesLanguageCorrection = true
request.recognitionLanguages = ["vi-VN", "en-US"]

let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])

do {
  try handler.perform([request])
} catch {
  fputs("Error: \(error.localizedDescription)\n", stderr)
  exit(1)
}
