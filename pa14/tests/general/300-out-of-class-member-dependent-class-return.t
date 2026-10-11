template<class T>
struct result
{
  int value;
};

template<class Tag, class Char>
struct holder
{
  static result<Char> call()
  {
    return value();
  }

  static result<Char> value();
};

template<class Tag, class Char>
result<Char> holder<Tag, Char>::value()
{
  result<Char> out = {0};
  return out;
}

struct tag {};

int main()
{
  return holder<tag, char>::call().value;
}
