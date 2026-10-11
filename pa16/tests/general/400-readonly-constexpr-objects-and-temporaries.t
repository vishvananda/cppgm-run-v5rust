// lowir.md storage=readonly: a constexpr object is a const object; the
// temporary a constant-initialized reference is bound to states readonly by
// its own type: `const int& r = 3` binds a const int, `int&& r = 3` an int, and
// a class prvalue's temporary (`const Point& r = Point{...}`) a Point.
struct Point
{
  int x, y;
  constexpr Point(int a, int b) : x(a), y(b) {}
};

constexpr Point origin(0, 1);
constexpr int squares[3] = {1, 4, 9};
constexpr const int& constant_reference = 3;
const int& const_reference = 4;
int&& rvalue_reference = 5;
const Point& class_reference = Point(6, 7);

int main()
{
  rvalue_reference = 8;
  return origin.y + squares[2] + constant_reference + const_reference + rvalue_reference +
    class_reference.y == 1 + 9 + 3 + 4 + 8 + 7 ? 0 : 1;
}
