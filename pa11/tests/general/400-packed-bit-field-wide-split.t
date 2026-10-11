// A packed bit-field spanning more than 8 bytes -- a 64-bit field starting
// past its first byte's start -- is read and written piecewise as the bytes
// it spans, through a 128-bit unit; its neighbours keep their values.
#pragma pack(push, 1)
struct Spans { char low : 3; unsigned long long wide : 64; char after; };
struct Signed { unsigned char head : 7; long value : 62; unsigned char tail : 3; };
#pragma pack(pop)

Spans global = { 2, 0x0123456789abcdefULL, 5 };

int main()
{
  if (sizeof(Spans) != 10 || sizeof(Signed) != 9) return 1;
  if (global.low != 2 || global.wide != 0x0123456789abcdefULL || global.after != 5)
    return 2;
  Spans spans;
  spans.low = -3;
  spans.after = 7;
  spans.wide = 0xfedcba9876543210ULL;
  if (spans.low != -3 || spans.wide != 0xfedcba9876543210ULL || spans.after != 7)
    return 3;
  spans.wide = 1;
  const unsigned char* bytes = reinterpret_cast<const unsigned char*>(&spans);
  if (bytes[0] != 0x0d || bytes[1] != 0 || bytes[7] != 0 || (bytes[8] & 7) != 0)
    return 4;
  ++spans.wide;
  if (spans.wide != 2 || spans.low != -3 || spans.after != 7) return 5;
  Signed value;
  value.head = 100;
  value.tail = 6;
  value.value = -123456789012345L;
  if (value.head != 100 || value.value != -123456789012345L || value.tail != 6)
    return 6;
  value.value = (1L << 60) + 3;
  if (value.value != (1L << 60) + 3 || value.head != 100 || value.tail != 6)
    return 7;
  return 0;
}
