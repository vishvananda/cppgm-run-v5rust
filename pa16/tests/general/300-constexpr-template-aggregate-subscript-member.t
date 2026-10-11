void subscript_out_of_bounds();

template<class T, unsigned N>
struct array
{
  T elems[N];

  constexpr T const & operator[](unsigned index) const
  {
    return __builtin_expect(!((index < N) && "in range"), 0) ?
        subscript_out_of_bounds() : (void)0,
        elems[index];
  }
};

template<class T>
void check()
{
  constexpr array<T, 2> values = {{3, 5}};
  static_assert(values[0] == 3, "");
  static_assert(values[1] == 5, "");

  constexpr array<T, 4> flat_values = {7, 11, 13, 17};
  static_assert(flat_values[0] == 7, "");
  static_assert(flat_values[1] == 11, "");
  static_assert(flat_values[2] == 13, "");
  static_assert(flat_values[3] == 17, "");
}

int main()
{
  check<int>();
}
