namespace VTracer.Interop;

public enum Clustering { ColorCluster = 0, Binary = 1, Watershed = 2 }
public enum Hierarchical { Stacked = 0, Cutout = 1 }
public enum FitMode { Pixel = 0, Polygon = 1, Spline = 2 }

/// <summary>Tracing settings, initialized to the Rust library's defaults.</summary>
public sealed class VTracerConfig
{
    public Clustering Clustering { get; set; } = Clustering.ColorCluster;
    public Hierarchical Hierarchical { get; set; } = Hierarchical.Stacked;
    public FitMode Mode { get; set; } = FitMode.Spline;
    public uint FilterSpeckle { get; set; } = 4;
    public int ColorPrecision { get; set; } = 6;
    public int LayerDifference { get; set; } = 16;
    public int CornerThreshold { get; set; } = 60;
    public int SpliceThreshold { get; set; } = 45;
    public double LengthThreshold { get; set; } = 4;
    public uint MaxIterations { get; set; } = 10;

    /// <summary>Coordinate decimal places (0..15), or null for automatic precision.</summary>
    public int? PathPrecision { get; set; } = 2;

    /// <summary>Simplification tolerance in pixels; null or zero disables it.</summary>
    public double? Simplify { get; set; }

    /// <summary>Maximum colors; null or zero disables quantization.</summary>
    public uint? MaxColors { get; set; }

    public uint Optimize { get; set; } = 1;
    public uint BinaryThreshold { get; set; } = 128;
    public bool BinaryAdaptive { get; set; }
    public uint BinaryAdaptiveWindow { get; set; }
    public uint WatershedDetail { get; set; } = 128;
    public double BinaryAdaptiveT { get; set; } = 15;

    /// <summary>Optional fixed palette of 0xRRGGBB colors. Takes priority over MaxColors.</summary>
    public uint[] Palette { get; set; } = System.Array.Empty<uint>();
}
