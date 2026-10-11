// VALIDATION: compile-pass

template<class T>
T && declval();

template<class From, class To>
struct is_same
{
  static constexpr bool value = false;
};

template<class T>
struct is_same<T, T>
{
  static constexpr bool value = true;
};

namespace source
{
template<class C>
auto sequence_begin(C & value) -> decltype(value.begin());

template<class C>
auto sequence_begin(C const & value) -> decltype(value.begin());
}

namespace alias = source;

struct sequence
{
  int const * begin() const;
};

static_assert(is_same<
    decltype(*alias::sequence_begin(declval<sequence const &>())),
    int const &>::value, "");

int main()
{
  return 0;
}
