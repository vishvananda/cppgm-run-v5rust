// expect exit 0: straddling constant fields initialize before a dynamic observer.
#pragma pack(push, 1)
struct Packed {
  unsigned char tag : 3;
  unsigned long long bits : 64;
  char tail;
};
struct Mixed {
  unsigned short first : 5;
  unsigned long long second : 61;
  signed char tail;
};
struct Wrapped {
  char before;
  Packed entries[2];
};
#pragma pack(pop)

extern Packed values[2];
extern Mixed mixed;
extern Packed zero;
extern Wrapped wrapped;

int observe() {
  return values[0].tag == 5 && values[0].bits == 0xfedcba9876543210ULL &&
    values[0].tail == 11 && values[1].tag == 2 &&
    values[1].bits == 0x0123456789abcdefULL && values[1].tail == 7 &&
    mixed.first == 23 && mixed.second == 0x123456789abcdefULL &&
    mixed.tail == -9 && zero.tag == 0 && zero.bits == 0 && zero.tail == 0 &&
    wrapped.before == 9 && wrapped.entries[1].tag == 2 &&
    wrapped.entries[1].bits == 0x0123456789abcdefULL &&
    wrapped.entries[1].tail == 7;
}

int snapshot = observe();
Packed values[2] = {{5, 0xfedcba9876543210ULL, 11},
                    {2, 0x0123456789abcdefULL, 7}};
Mixed mixed = {23, 0x123456789abcdefULL, -9};
Packed zero = Packed();
Wrapped wrapped = {9, {{5, 0xfedcba9876543210ULL, 11},
                      {2, 0x0123456789abcdefULL, 7}}};

int main() { return snapshot != 1 || !observe(); }
