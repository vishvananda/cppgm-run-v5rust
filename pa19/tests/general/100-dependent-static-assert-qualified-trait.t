template<class T>
struct trait {
  static const bool value = false;
};

template<>
struct trait<int> {
  static const bool value = true;
};

template<class T>
struct box {
  static_assert(trait<T>::value, "dependent trait should be checked after substitution");

  int value() const
  {
    return 0;
  }
};

int main()
{
  box<int> b;
  return b.value();
}
