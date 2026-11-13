#nullable enable

using System;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace VTracer.Interop;

/// <summary>
/// Converts RGBA8 pixels to SVG using vtracer_native.dll.
/// Requires .NET 7+ and AllowUnsafeBlocks enabled in the consuming project.
/// </summary>
public static partial class VTracerNative
{
    /// <summary>
    /// Converts tightly packed RGBA bytes (not BGRA or encoded PNG/JPEG data).
    /// The span must contain exactly width * height * 4 bytes, with no row padding.
    /// </summary>
    public static string TraceRgba(ReadOnlySpan<byte> pixels, uint width, uint height)
        => TraceRgbaCore(pixels, width, height, null);

    /// <summary>Converts tightly packed RGBA8 pixels using custom tracing settings.</summary>
    public static string TraceRgba(
        ReadOnlySpan<byte> pixels, uint width, uint height, VTracerConfig config)
    {
        ArgumentNullException.ThrowIfNull(config);
        return TraceRgbaCore(pixels, width, height, config);
    }

    private static unsafe string TraceRgbaCore(
        ReadOnlySpan<byte> pixels, uint width, uint height, VTracerConfig? config)
    {
        if (width == 0 || width > int.MaxValue)
            throw new ArgumentOutOfRangeException(nameof(width));
        if (height == 0 || height > int.MaxValue)
            throw new ArgumentOutOfRangeException(nameof(height));
        if ((ulong)width * height * 4 != (ulong)pixels.Length)
            throw new ArgumentException("Expected width * height * 4 RGBA bytes.", nameof(pixels));

        nint result = 0;
        try
        {
            int status;
            uint[] palette = config?.Palette ?? Array.Empty<uint>();
            fixed (byte* pointer = pixels)
            fixed (uint* palettePointer = palette)
            {
                if (config is null)
                {
                    status = TraceRgbaNative(pointer, (nuint)pixels.Length, width, height, out result);
                }
                else
                {
                    var nativeConfig = new NativeConfig
                    {
                        Clustering = (int)config.Clustering,
                        Hierarchical = (int)config.Hierarchical,
                        Mode = (int)config.Mode,
                        FilterSpeckle = config.FilterSpeckle,
                        ColorPrecision = config.ColorPrecision,
                        LayerDifference = config.LayerDifference,
                        CornerThreshold = config.CornerThreshold,
                        SpliceThreshold = config.SpliceThreshold,
                        LengthThreshold = config.LengthThreshold,
                        MaxIterations = config.MaxIterations,
                        PathPrecision = config.PathPrecision ?? -1,
                        Simplify = config.Simplify ?? 0,
                        MaxColors = config.MaxColors ?? 0,
                        Optimize = config.Optimize,
                        BinaryThreshold = config.BinaryThreshold,
                        BinaryAdaptive = config.BinaryAdaptive ? 1u : 0u,
                        BinaryAdaptiveWindow = config.BinaryAdaptiveWindow,
                        WatershedDetail = config.WatershedDetail,
                        BinaryAdaptiveT = config.BinaryAdaptiveT,
                        Palette = palettePointer,
                        PaletteLength = (nuint)palette.Length,
                    };
                    status = TraceRgbaWithConfigNative(
                        pointer, (nuint)pixels.Length, width, height, in nativeConfig, out result);
                }
            }

            string text = Marshal.PtrToStringUTF8(result)
                ?? throw new InvalidOperationException("vtracer returned no result.");
            if (status != 0)
            {
                if (status == 1 && config is not null)
                    throw new ArgumentException(text, nameof(config));
                throw new InvalidOperationException(text);
            }
            return text;
        }
        finally
        {
            if (result != 0)
                FreeString(result);
        }
    }

    [LibraryImport("vtracer_native", EntryPoint = "vtracer_trace_rgba")]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static unsafe partial int TraceRgbaNative(
        byte* pixels, nuint length, uint width, uint height, out nint result);

    [LibraryImport("vtracer_native", EntryPoint = "vtracer_trace_rgba_with_config")]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static unsafe partial int TraceRgbaWithConfigNative(
        byte* pixels, nuint length, uint width, uint height, in NativeConfig config, out nint result);

    [StructLayout(LayoutKind.Sequential)]
    private unsafe struct NativeConfig
    {
        public int Clustering, Hierarchical, Mode;
        public uint FilterSpeckle;
        public int ColorPrecision, LayerDifference, CornerThreshold, SpliceThreshold;
        public double LengthThreshold;
        public uint MaxIterations;
        public int PathPrecision;
        public double Simplify;
        public uint MaxColors, Optimize, BinaryThreshold, BinaryAdaptive, BinaryAdaptiveWindow, WatershedDetail;
        public double BinaryAdaptiveT;
        public uint* Palette;
        public nuint PaletteLength;
    }

    [LibraryImport("vtracer_native", EntryPoint = "vtracer_free_string")]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    private static partial void FreeString(nint value);
}
