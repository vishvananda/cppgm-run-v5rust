template<class T>
struct Stream
{
  int value;
};

template<class T>
int finish(Stream<T> &stream)
{
  return stream.value + 3;
}

struct Sink
{
  int value;

  Sink &operator<<(int (*fn)(Stream<char> &))
  {
    Stream<char> stream;
    stream.value = value;
    value = fn(stream);
    return *this;
  }
};

int main()
{
  Sink sink;
  sink.value = 4;
  sink << finish;
  return sink.value == 7 ? 0 : 1;
}
