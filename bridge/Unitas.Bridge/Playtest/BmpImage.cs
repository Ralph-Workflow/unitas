using System;
using System.Buffers.Binary;
using System.Globalization;
using System.IO;
using UnityEngine;

namespace Unitas.Bridge.Playtest
{
    public sealed class BmpImage
    {
        private const int FileHeaderSize = 14;
        private const int InfoHeaderSize = 40;
        private const int HeaderSize = FileHeaderSize + InfoHeaderSize;
        private const int ExpectedPlanes = 1;
        private const int ExpectedBitDepth = 24;
        private const int ExpectedCompression = 0;
        private const int BytesPerPixel = 3;

        private readonly byte[] _data;
        private readonly int _dataOffset;
        private readonly int _rowStride;

        private BmpImage(int width, int height, int dataOffset, int rowStride, byte[] data)
        {
            Width = width;
            Height = height;
            _dataOffset = dataOffset;
            _rowStride = rowStride;
            _data = data;
        }

        public int Width { get; }

        public int Height { get; }

        public static BmpImage Load(string path)
        {
            ArgumentNullException.ThrowIfNull(path);
            byte[] data = File.ReadAllBytes(path);
            if (data.Length < HeaderSize)
            {
                throw new InvalidDataException("BMP file is truncated: expected at least " + HeaderSize.ToString(CultureInfo.InvariantCulture) + " bytes, found " + data.Length.ToString(CultureInfo.InvariantCulture) + ".");
            }

            if (data[0] != (byte)'B' || data[1] != (byte)'M')
            {
                throw new InvalidDataException("Unexpected BMP signature: expected 'BM'.");
            }

            int dataOffset = (int)BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(10, 4));
            int headerSize = (int)BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(14, 4));
            if (headerSize != InfoHeaderSize)
            {
                throw new InvalidDataException("Unexpected BMP DIB header size: expected " + InfoHeaderSize.ToString(CultureInfo.InvariantCulture) + ", found " + headerSize.ToString(CultureInfo.InvariantCulture) + ".");
            }

            int width = BinaryPrimitives.ReadInt32LittleEndian(data.AsSpan(18, 4));
            int rawHeight = BinaryPrimitives.ReadInt32LittleEndian(data.AsSpan(22, 4));
            int planes = BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(26, 2));
            int bitDepth = BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(28, 2));
            int compression = (int)BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(30, 4));
            if (planes != ExpectedPlanes)
            {
                throw new InvalidDataException("Unexpected BMP plane count: expected " + ExpectedPlanes.ToString(CultureInfo.InvariantCulture) + ", found " + planes.ToString(CultureInfo.InvariantCulture) + ".");
            }

            if (bitDepth != ExpectedBitDepth)
            {
                throw new InvalidDataException("Unexpected BMP bit depth: expected " + ExpectedBitDepth.ToString(CultureInfo.InvariantCulture) + ", found " + bitDepth.ToString(CultureInfo.InvariantCulture) + ".");
            }

            if (compression != ExpectedCompression)
            {
                throw new InvalidDataException("Unexpected BMP compression: expected " + ExpectedCompression.ToString(CultureInfo.InvariantCulture) + ", found " + compression.ToString(CultureInfo.InvariantCulture) + ".");
            }

            if (width <= 0)
            {
                throw new InvalidDataException("Unexpected BMP width: " + width.ToString(CultureInfo.InvariantCulture) + ".");
            }

            if (rawHeight == 0)
            {
                throw new InvalidDataException("Unexpected BMP height: 0.");
            }

            if (dataOffset < HeaderSize)
            {
                throw new InvalidDataException("Unexpected BMP pixel data offset: " + dataOffset.ToString(CultureInfo.InvariantCulture) + ".");
            }

            int height = Math.Abs(rawHeight);
            int rowStride = ((width * BytesPerPixel) + 3) & ~3;
            long required = (long)rowStride * height;
            long available = data.Length - dataOffset;
            if (available < required)
            {
                throw new InvalidDataException("BMP pixel data is truncated: expected " + required.ToString(CultureInfo.InvariantCulture) + " bytes, found " + available.ToString(CultureInfo.InvariantCulture) + ".");
            }

            return new BmpImage(width, height, dataOffset, rowStride, data);
        }

        public int CountPixelsDifferentFrom(Color background)
        {
            byte blue = ToByte(background.b);
            byte green = ToByte(background.g);
            byte red = ToByte(background.r);
            int different = 0;
            for (int row = 0; row < Height; row++)
            {
                int rowStart = _dataOffset + (row * _rowStride);
                for (int column = 0; column < Width; column++)
                {
                    int pixel = rowStart + (column * BytesPerPixel);
                    if (_data[pixel] != blue || _data[pixel + 1] != green || _data[pixel + 2] != red)
                    {
                        different++;
                    }
                }
            }

            return different;
        }

        private static byte ToByte(float value)
        {
            if (float.IsNaN(value))
            {
                return 0;
            }

            float clamped = value < 0f ? 0f : (value > 1f ? 1f : value);
            return (byte)Math.Round(clamped * 255f, MidpointRounding.AwayFromZero);
        }
    }
}
