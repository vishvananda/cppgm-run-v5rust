// VALIDATION: compile-pass
// N3485 focus: 6.8 [stmt.ambig], 14.2 [temp.names]

namespace helper
{
template<class T>
int id(T value)
{
  return value;
}
}

struct holder
{
  int value;

  explicit holder(int v) : value(v)
  {
  }
};

template<class T>
int run(T value)
{
  holder local(helper::id<T>(value));
  return local.value;
}

int main()
{
  return run<int>(5) == 5 ? 0 : 1;
}
