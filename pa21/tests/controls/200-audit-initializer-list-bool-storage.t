// expect exit 0
namespace std { template<class T> class initializer_list; }
int bits(int x, int y) {
  std::initializer_list<bool> values = {x < y, {x == y}, x && y, !x, bool(y)};
  int result = 0;
  for (bool value : values) result = result * 2 + value;
  return result;
}
struct Pick {
  int bits;
  Pick(std::initializer_list<bool> values) : bits(0) {
    for (bool value : values) bits = bits * 2 + value;
  }
};
int main() {
  if (bits(1, 2) != 21 || bits(0, 0) != 10) return 1;
  int x = 3;
  Pick pick{x < 4, x == 4, bool(x)};
  return pick.bits == 5 ? 0 : 2;
}
