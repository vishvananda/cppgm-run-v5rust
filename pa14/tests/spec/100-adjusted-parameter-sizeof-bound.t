template<class T>
int adjusted(T array[sizeof(T)], int (&bytes)[sizeof(array)])
{
  return array[0] + sizeof(bytes);
}

int increment(int value) { return value + 1; }

int callback_size(int callback(int), int (&bytes)[sizeof(callback)])
{
  return callback(3) + sizeof(bytes);
}

int qualification(int *) { return 1; }
int qualification(const int *) { return 0; }

int qualified(const int value, int (&bytes)[sizeof(value)])
{
  return qualification(&value);
}

int main()
{
  int array[4] = {5};
  int pointer_bytes[sizeof(void *)] = {};
  int value_bytes[sizeof(int)] = {};
  if (adjusted<int>(array, pointer_bytes) != 5 + sizeof(pointer_bytes)) return 1;
  if (callback_size(increment, pointer_bytes) != 4 + sizeof(pointer_bytes)) return 2;
  return qualified(3, value_bytes);
}
