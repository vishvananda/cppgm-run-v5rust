// N3485 5.19p5, 6.4.2p2, 7.6.2, 8.3.4p1, 9.6p3: expect exit 0
struct Width { constexpr operator int() const { return 3; } };
struct Size { constexpr operator unsigned long() const { return 4; } };
struct Alignment { constexpr operator int() const { return 16; } };
struct Label {
  constexpr operator int() const { return 7; }
  constexpr operator long() const { return 8; }
};
enum Value { width = Width() };
enum class Tag { selected };
struct Policy { static constexpr Tag tag = Tag::selected; };
constexpr Tag tag = Policy::tag;
static_assert(tag == Policy::tag, "unqualified enum values");
int values[Size()];
struct alignas(Alignment{}) Bits { unsigned value : Width(); };
int pick(long n) {
  switch (n) { case Label(): return 9; }
  return 0;
}
int main() {
  Bits bits = {5};
  return width == 3 && sizeof(values) == 16 && alignof(Bits) == 16 &&
         bits.value == 5 && pick(8) == 9 ? 0 : 1;
}
