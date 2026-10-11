constexpr int square(int n) { return n * n; }

struct Point
{
  int value;
  constexpr Point(int n) : value(n) {}
  constexpr Point operator-() const { return Point(-value); }
};

struct Derived : Point
{
  constexpr Derived(Point source) : Point(source) {}
};

extern const Point point;
extern const Point negative;
extern const Derived derived;
extern const Point elements[2];

volatile int late_calls = 0;
int late() { ++late_calls; return 13; }
struct Fallback
{
  int value;
  constexpr Fallback(int n) : value(n ? n : late()) {}
};
extern Fallback dynamic;

struct Observer
{
  bool valid;
  Observer() : valid(point.value == 9 && negative.value == -9 &&
                     derived.value == 9 && elements[0].value == 3 &&
                     elements[1].value == 5 &&
                     late_calls == 0 && dynamic.value == 0) {}
};

Observer observer;
constexpr Point point(square(3));
constexpr Point negative = -point;
constexpr Derived derived(point);
constexpr Point elements[2] = {Point(3), Point(5)};
Fallback dynamic(0);

int main()
{
  return observer.valid && dynamic.value == 13 && late_calls == 1 ? 0 : 1;
}
