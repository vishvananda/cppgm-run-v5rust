// VALIDATION: a member function template of a class template calls another
// member template of the same class with explicit arguments; each
// specialization of the class resolves that call to its own member, not to
// the member a sibling specialization instantiated first.

template <class T>
T&& declval();

template <class H>
struct leaf
{
  H value_;

  template <class T>
  static constexpr int width()
  {
    return static_cast<int>(sizeof(H) * 1000 + sizeof(T));
  }

  template <class T>
  explicit leaf(T&& t) : value_(static_cast<T&&>(t))
  {
    static_assert(width<T&&>() > 0, "width is positive");
  }

  template <class T>
  int measure(T&&) const { return width<T&&>(); }
};

struct wide
{
  char bytes[16];
};

int main()
{
  leaf<int> narrow(7);
  wide payload;
  leaf<wide> broad(payload);
  if (narrow.measure(1) != 4 * 1000 + 4) return 1;
  if (broad.measure(1) != 16 * 1000 + 4) return 2;
  if (narrow.measure(payload) != 4 * 1000 + 16) return 3;
  return 0;
}
