template<class T, class V>
T & unsafe_get(V & value) { return value; }

template<int I, class V>
int & unsafe_get(V &);

template<class T, class V>
auto get(V & value) -> decltype(unsafe_get<T>(value))
{
  return unsafe_get<T>(value);
}

int main()
{
  int value;
  return &get<int>(value) != &value;
}
