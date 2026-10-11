// VALIDATION: compile-pass

template<class T, T v>
struct integral_constant
{
  static constexpr T value = v;
};

typedef unsigned long size_type;

struct array
{
  struct table
  {
    unsigned long size;
  };

  static constexpr size_type max_size() noexcept;
};

constexpr size_type
array::max_size() noexcept
{
  using min = integral_constant<size_type,
      (size_type(-1) - sizeof(table)) / sizeof(int)>;
  return min::value < 64 ? min::value : 64;
}

struct handler
{
  static constexpr size_type max_array_size = array::max_size();
  int value;
};

struct parser
{
  handler h;
};

int main()
{
  return sizeof(parser) > 0 && handler::max_array_size == 64 ? 0 : 1;
}
