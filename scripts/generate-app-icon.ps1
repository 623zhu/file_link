# Generates a 1024x1024 File Link source icon (WPF, STA).
Add-Type -AssemblyName PresentationCore, PresentationFramework, WindowsBase

function New-ColorBrush([byte]$r, [byte]$g, [byte]$b) {
    $c = [System.Windows.Media.Color]::FromRgb($r, $g, $b)
    $brush = New-Object System.Windows.Media.SolidColorBrush $c
    $brush.Freeze()
    $brush
}

function New-RoundRect([double]$x, [double]$y, [double]$w, [double]$h, [double]$radius) {
    $g = New-Object System.Windows.Media.RectangleGeometry
    $g.Rect = New-Object System.Windows.Rect $x, $y, $w, $h
    $g.RadiusX = $radius
    $g.RadiusY = $radius
    $g.Freeze()
    $g
}

$size = 1024
$dv = New-Object System.Windows.Media.DrawingVisual
$dc = $dv.RenderOpen()

$blue = New-ColorBrush 37 99 235       # #2563EB — matches app accent
$blueDark = New-ColorBrush 29 78 216   # #1D4ED8
$white = New-ColorBrush 255 255 255
$line = New-ColorBrush 191 219 254     # #BFDBFE

# Rounded square background with slight padding so the glyph survives icon masks.
$pad = 48.0
$dc.DrawGeometry($blue, $null, (New-RoundRect $pad $pad ($size - 2 * $pad) ($size - 2 * $pad) 220))

# Document body
$docX = 292.0
$docY = 188.0
$docW = 440.0
$docH = 568.0
$docRadius = 52.0
$fold = 148.0

$docGeo = New-RoundRect $docX $docY $docW $docH $docRadius

$foldCut = New-Object System.Windows.Media.PathGeometry
$cutFig = New-Object System.Windows.Media.PathFigure
$cutFig.StartPoint = New-Object System.Windows.Point ($docX + $docW - $fold), $docY
$cutFig.IsClosed = $true
$cutFig.IsFilled = $true
$cutSeg = New-Object System.Windows.Media.PolyLineSegment
[void]$cutSeg.Points.Add((New-Object System.Windows.Point ($docX + $docW), $docY))
[void]$cutSeg.Points.Add((New-Object System.Windows.Point ($docX + $docW), ($docY + $fold)))
$cutFig.Segments.Add($cutSeg)
$foldCut.Figures.Add($cutFig)
$foldCut.Freeze()

$body = New-Object System.Windows.Media.CombinedGeometry
$body.GeometryCombineMode = [System.Windows.Media.GeometryCombineMode]::Exclude
$body.Geometry1 = $docGeo
$body.Geometry2 = $foldCut
$body.Freeze()
$dc.DrawGeometry($white, $null, $body)

# Folded corner
$foldFace = New-Object System.Windows.Media.PathGeometry
$foldFig = New-Object System.Windows.Media.PathFigure
$foldFig.StartPoint = New-Object System.Windows.Point ($docX + $docW - $fold), $docY
$foldFig.IsClosed = $true
$foldFig.IsFilled = $true
$foldSeg = New-Object System.Windows.Media.PolyLineSegment
[void]$foldSeg.Points.Add((New-Object System.Windows.Point ($docX + $docW - $fold), ($docY + $fold)))
[void]$foldSeg.Points.Add((New-Object System.Windows.Point ($docX + $docW), ($docY + $fold)))
$foldFig.Segments.Add($foldSeg)
$foldFace.Figures.Add($foldFig)
$foldFace.Freeze()
$dc.DrawGeometry($blueDark, $null, $foldFace)

# Content lines — read as a file even at small sizes
$pen = New-Object System.Windows.Media.Pen $line, 32
$pen.StartLineCap = [System.Windows.Media.PenLineCap]::Round
$pen.EndLineCap = [System.Windows.Media.PenLineCap]::Round
$pen.Freeze()
$left = $docX + 78
$right = $docX + $docW - 78
$dc.DrawLine($pen, (New-Object System.Windows.Point $left, 430), (New-Object System.Windows.Point $right, 430))
$dc.DrawLine($pen, (New-Object System.Windows.Point $left, 520), (New-Object System.Windows.Point $right, 520))
$dc.DrawLine($pen, (New-Object System.Windows.Point $left, 610), (New-Object System.Windows.Point ($right - 90), 610))

# Link badge (chain of two rings) overlapping the lower-right of the document
function Add-Ring([double]$cx, [double]$cy, [double]$rx, [double]$ry, [double]$thickness) {
    $outer = New-Object System.Windows.Media.EllipseGeometry
    $outer.Center = New-Object System.Windows.Point $cx, $cy
    $outer.RadiusX = $rx
    $outer.RadiusY = $ry
    $inner = New-Object System.Windows.Media.EllipseGeometry
    $inner.Center = New-Object System.Windows.Point $cx, $cy
    $inner.RadiusX = ($rx - $thickness)
    $inner.RadiusY = ($ry - $thickness)
    $ring = New-Object System.Windows.Media.CombinedGeometry
    $ring.GeometryCombineMode = [System.Windows.Media.GeometryCombineMode]::Exclude
    $ring.Geometry1 = $outer
    $ring.Geometry2 = $inner
    $ring.Freeze()
    $ring
}

$badgeCx = 730.0
$badgeCy = 730.0
$badgeR = 168.0
$badge = New-Object System.Windows.Media.EllipseGeometry
$badge.Center = New-Object System.Windows.Point $badgeCx, $badgeCy
$badge.RadiusX = $badgeR
$badge.RadiusY = $badgeR
$badge.Freeze()
$dc.DrawGeometry($white, $null, $badge)

# Two interlocking rings in accent blue
$dc.PushTransform((New-Object System.Windows.Media.RotateTransform 35, $badgeCx, $badgeCy))
$dc.DrawGeometry($blue, $null, (Add-Ring ($badgeCx - 38) $badgeCy 70 44 22))
$dc.DrawGeometry($blue, $null, (Add-Ring ($badgeCx + 38) $badgeCy 70 44 22))
$dc.Pop()

$dc.Close()

$bmp = New-Object System.Windows.Media.Imaging.RenderTargetBitmap(
    $size, $size, 96, 96,
    [System.Windows.Media.PixelFormats]::Pbgra32
)
$bmp.Render($dv)

$out = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\app-icon.png"))
$encoder = New-Object System.Windows.Media.Imaging.PngBitmapEncoder
$encoder.Frames.Add([System.Windows.Media.Imaging.BitmapFrame]::Create($bmp))
$fs = [System.IO.File]::Open($out, "Create")
$encoder.Save($fs)
$fs.Close()

$png = Get-Item $out
Write-Output "wrote $($png.FullName) ($($png.Length) bytes)"
