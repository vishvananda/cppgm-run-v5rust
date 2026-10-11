template<class T, unsigned N>
struct array
{
  T elems[N];

  constexpr T const * data() const
  {
    return elems;
  }

  constexpr T const * begin() const
  {
    return elems;
  }

  constexpr T const & operator[](unsigned index) const
  {
    return elems[index];
  }
};

template<class T, unsigned N>
constexpr bool separate_parameter_storage(array<T, N> copy,
                                          array<T, N> const & source)
{
  return copy.data() != source.data();
}

void check()
{
  constexpr array<int, 2> values = {{3, 5}};
  static_assert(values.data() == values.elems, "");
  static_assert(values.begin() == values.data(), "");
  static_assert(&values[0] == values.data(), "");
  static_assert(&values[1] == values.data() + 1, "");

  constexpr array<int, 2> copied = values;
  static_assert(copied.data() != values.data(), "");
  static_assert(separate_parameter_storage(values, values), "");
}

constexpr int const * null_data()
{
  return 0;
}

int main()
{
  check();
  static_assert(null_data() == 0, "");
  static_assert(null_data() + 0 == 0, "");
}
