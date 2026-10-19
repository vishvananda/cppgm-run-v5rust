// expect exit 0
namespace std { template<class T> class initializer_list; }
int live, tokens;
bool choose;
struct Token { Token() { ++tokens; } ~Token() { --tokens; } };
struct Plain { Plain(int) { ++live; } ~Plain() { --live; } };
struct Item {
  Item(int, Token = Token()) { ++live; }
  ~Item() noexcept(false) { --live; }
};
using List = std::initializer_list<Plain>;
List returned() { Token t; return {1, 2}; }
struct Pick {
  int sum;
  Pick(std::initializer_list<int> x, Token = Token()) : sum(0) {
    for (int n : x) sum += n;
  }
};
void take(std::initializer_list<int>, Token = Token()) {}
void nested(bool fail) {
  Token t;
  try { throw 1; }
  catch (int) {
    try {
      List x = {1, 2, 3, 4, 5, 6, 7, 8, 9};
      if (fail) throw 2;
    } catch (...) {}
  }
}
void conditional() {
  Token t;
  const List& x = choose ? List{1,2,3,4,5,6,7,8,9} : List{9,8,7,6,5,4,3,2,1};
  (void)x;
  using Items = std::initializer_list<Item>;
  const Items& y = choose ? Items{1,2,3,4,5,6,7,8} : Items{8,7,6,5,4,3,2,1};
  (void)y;
}
int main() {
  { Token t; Pick p{1,2,3}; take({1,2,3}); if (p.sum != 6 || tokens != 1) return 1; }
  returned();
  nested(false); nested(true);
  choose = false; conditional();
  choose = true; conditional();
  return live == 0 && tokens == 0 ? 0 : 2;
}
