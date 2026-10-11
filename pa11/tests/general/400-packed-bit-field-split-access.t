// A packed bit-field that no single 1-, 2-, 4- or 8-byte access within its
// class holds is read and written piecewise as the bytes it spans (as g++
// does): Packed::wide spans bytes 0-4 of a 5-byte class, Tail::value bytes
// 1-8 of a 10-byte one.  Neighbouring fields and bytes keep their values.
#pragma pack(push, 1)
struct Packed { short low : 3; int wide : 30; };
struct Tail { char first; long value : 60; char last; };
struct Mixed { unsigned char small : 5; unsigned long long middle : 40; unsigned high : 9; };
#pragma pack(pop)

Packed global = { 2, 123456789 };

int main()
{
  if (sizeof(Packed) != 5 || sizeof(Tail) != 10 || sizeof(Mixed) != 7) return 1;
  if (global.low != 2 || global.wide != 123456789) return 2;
  Packed packed;
  packed.low = -1;
  packed.wide = -123456789;
  if (packed.low != -1 || packed.wide != -123456789) return 3;
  packed.wide = 0x1fffffff;
  packed.low = 3;
  const unsigned char* bytes = reinterpret_cast<const unsigned char*>(&packed);
  if (bytes[0] != 0xfb || bytes[1] != 0xff || bytes[2] != 0xff ||
      bytes[3] != 0xff || bytes[4] != 0x00) return 4;
  Tail tail;
  tail.first = 1;
  tail.last = 2;
  tail.value = -5;
  if (tail.first != 1 || tail.value != -5 || tail.last != 2) return 5;
  tail.value = 0x05bcdef012345678L;
  if (tail.value != 0x05bcdef012345678L || tail.first != 1 || tail.last != 2) return 6;
  Mixed mixed;
  mixed.small = 31;
  mixed.middle = 0xffffffffffULL;
  mixed.high = 300;
  if (mixed.small != 31 || mixed.middle != 0xffffffffffULL || mixed.high != 300) return 7;
  mixed.middle = 0x123456789aULL;
  if (mixed.small != 31 || mixed.middle != 0x123456789aULL || mixed.high != 300) return 8;
  return 0;
}
