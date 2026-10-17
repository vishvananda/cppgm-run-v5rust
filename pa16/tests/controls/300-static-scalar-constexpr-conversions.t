// N3485 3.6.2p2, 5.19, 12.3.2: expect exit 0
struct Integer {
  int value;
  constexpr Integer(int n) : value(n) {}
  constexpr operator int() const { return value; }
};
struct Floating { constexpr operator double() const { return 3.5; } };
int object = 9;
struct Pointer { constexpr operator int*() const { return &object; } };
constexpr int number = Integer(7);
int ordinary = Integer(11);
constexpr double precise = Floating();
float rounded = Floating();
constexpr int* address = Pointer();
int* ordinary_address = Pointer();
bool truth = Pointer();
int calls;
struct Runtime { operator int() const { return ++calls; } };
int dynamic = Runtime();
int main() {
  return number == 7 && ordinary == 11 && precise == 3.5 && rounded == 3.5f &&
         address == &object && ordinary_address == address && truth &&
         dynamic == 1 && calls == 1 ? 0 : 1;
}
