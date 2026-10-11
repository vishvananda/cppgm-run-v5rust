template<class T>
struct family
{
  struct argument
  {
    T value;
  };
};

struct pick
{
  long value;

  pick(family<int>::argument) : value(1) {}
  pick(family<long>::argument) : value(2) {}
};

int main()
{
  family<long>::argument argument = {0};
  pick result(argument);
  return result.value == 2 ? 0 : 1;
}
