// N3485 5.19, 14.6, 14.7.1: expect exit 0
constexpr int choices[] = {2, 4};
template<int N> struct Value { static constexpr int value = N; };
template<class T> struct Query {
  static constexpr int count() { return sizeof(T); }
  typedef Value<count()> direct;
  typedef Value<choices[sizeof(T) == sizeof(int)]> indexed;
  void unused() { T::missing; }
};
static_assert(Query<int>::direct::value == sizeof(int), "demanded call");
static_assert(Query<int>::indexed::value == 4, "demanded array read");
static_assert(Query<char>::indexed::value == 2, "separate substitution");
int main() { return 0; }
