template <typename T>
struct holder {};

struct one { char data[1]; };
struct two { char data[2]; };

template <typename T>
one pick(holder<T> &)
{
  return one();
}

template <typename T>
two pick(holder<const T> &)
{
  return two();
}

int main()
{
  holder<const char> data;
  return sizeof(pick(data)) == sizeof(two) ? 0 : 1;
}
