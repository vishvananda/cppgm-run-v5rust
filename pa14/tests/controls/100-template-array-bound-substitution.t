// N3485 8.3.5p5, 14.6.2.3p2, 14.8.2.5p5: retain dependent bounds,
// adjusted prototype objects and result operands independently; expect exit 0
template<class T>
auto bytes(T array[sizeof(T)], int (&data)[sizeof(array)]) -> decltype(sizeof(T))
{
  return sizeof(data);
}

template<class T> int measure(const T (&data)[sizeof(T)][sizeof(T) + 1]);
template<class U> int measure(const U (&data)[sizeof(U)][sizeof(U) + 1])
{
  return sizeof(data);
}

template<class T> int (*result())[sizeof(T)] { return 0; }

template<class T> int local_arrays()
{
  T values[sizeof(T)] = {1, 2};
  char text[sizeof(T)] = "a";
  return sizeof(values) + sizeof(text);
}

int main()
{
  int array[4] = {};
  int pointer_bytes[sizeof(void *)] = {};
  int matrix[4][5] = {};
  if (bytes<int>(array, pointer_bytes) != sizeof(pointer_bytes)) return 1;
  if (measure(matrix) != sizeof(matrix)) return 2;
  if (local_arrays<int>() != sizeof(array) + sizeof(int)) return 3;
  return sizeof(*result<int>()) != sizeof(array);
}
