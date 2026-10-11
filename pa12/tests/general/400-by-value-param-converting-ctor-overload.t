// VALIDATION: compile-pass

struct Stream {
  int value;

  Stream() : value(5) {}
};

struct Iterator {
  int value;

  Iterator(Stream &stream) : value(stream.value) {}
};

int put(Iterator it, long value)
{
  return it.value == 5 && value == 7 ? 0 : 1;
}

int put(int, long)
{
  return 2;
}

int main()
{
  Stream stream;
  return put(stream, 7L);
}
