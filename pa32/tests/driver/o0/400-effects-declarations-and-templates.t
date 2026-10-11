// lowir.md effects (compared here, in the -O0 driver lane, because PA28 owns
// the attributes): a function states effects=readnone when any declaration of
// it carries GNU const and effects=readonly when one carries GNU pure and none
// const: a later declaration adds it and a definition without it keeps it; a
// function template's specializations take the template's, an explicit
// specialization only its own; a virtual call of such a function states it on
// its call signature; __builtin_strlen is readonly.
int counter = 0;
int later(int x);
int later(int x) __attribute__((const));
int later(int x) { return x + 1; }
int earlier(int x) __attribute__((pure));
int earlier(int x) { return x + counter; }
[[gnu::const]] int bracketed(int x) { return x * 2; }
template<class T> T doubled(T x) __attribute__((const));
template<class T> T doubled(T x) { return x + x; }
template<> int doubled<int>(int x) { ++counter; return x + x; }
struct Shape
{
  int sides;
  virtual int area() const __attribute__((pure));
  virtual ~Shape() {}
};
int Shape::area() const { return sides; }
unsigned long length(const char* text) { return __builtin_strlen(text); }

int main()
{
  Shape square;
  square.sides = 4;
  const Shape& view = square;
  int total = later(1) + earlier(1) + bracketed(1) + doubled(2.0) + doubled(3) +
    view.area() + static_cast<int>(length("abc"));
  return total == 2 + 1 + 2 + 4 + 6 + 4 + 3 ? 0 : 1;
}
