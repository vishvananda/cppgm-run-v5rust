template<class T>
struct IdentityBase : T {
};

template<class T>
bool read_value()
{
  return IdentityBase<T>::value;
}

struct TrueValue {
  static const bool value = true;
};

int main()
{
  return read_value<TrueValue>() ? 0 : 1;
}
