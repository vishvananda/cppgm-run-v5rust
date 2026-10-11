struct needs_arg
{
  explicit needs_arg(int) {}
};

template<class T>
struct storage
{
  T value;
  storage() = default;
};

static_assert(!__is_constructible(storage<needs_arg>), "");

int main()
{
  return 0;
}
