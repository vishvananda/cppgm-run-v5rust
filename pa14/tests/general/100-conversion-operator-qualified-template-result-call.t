namespace N {
template<class T>
struct Vec {};
}

template<class T>
struct Lazy {
  N::Vec<T> values;

  operator const N::Vec<T> &() const
  {
    return values;
  }

  operator N::Vec<T> &()
  {
    return values;
  }
};

void sink(N::Vec<int> &) {}

int main()
{
  Lazy<int> lazy;
  sink(lazy);
  return 0;
}
