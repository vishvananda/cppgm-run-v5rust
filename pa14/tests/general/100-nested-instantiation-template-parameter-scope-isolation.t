// VALIDATION: compile-pass
// Instantiating an inner template does not make an outer template parameter
// name own declarations in the inner template.

template<class U>
struct inner
{
  using T = int;
  T value;
};

template<class T>
struct outer
{
  inner<int> value;
};

int main()
{
  outer<long> object;
  object.value.value = 0;
  return object.value.value;
}
