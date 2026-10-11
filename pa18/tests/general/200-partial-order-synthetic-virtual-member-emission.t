template <typename T>
struct text_for;

template <>
struct text_for<int>
{
  int size()
  {
    return 1;
  }
};

template <>
struct text_for<const int>
{
  int size()
  {
    return 2;
  }
};

template <typename T>
struct cache
{
  virtual int fill();
};

template <typename T>
int cache<T>::fill()
{
  text_for<T> text;
  return text.size();
}

template <typename T>
struct holder
{
  cache<T> stored;
};

struct one
{
  char data[1];
};

struct two
{
  char data[2];
};

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
  holder<const int> h;
  return sizeof(pick(h)) == sizeof(two) ? 0 : 1;
}
