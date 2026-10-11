template<class T>
struct box
{
  int value;

  box() : value(0) {}

  template<class U>
  box(const box<U> &other) : value(other.value + 1) {}

  template<class U>
  explicit box(box<U> other) : value(other.value + 100) {}
};

struct type {};

int main()
{
  box<type> source;
  source.value = 41;
  box<const type> result = source;
  return result.value == 42 ? 0 : 1;
}
