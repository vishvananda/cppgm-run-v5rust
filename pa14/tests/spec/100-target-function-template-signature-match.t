// VALIDATION: compile-pass
// N3485 focus: 14.8.2.2 [temp.deduct.funcaddr]

struct Sink
{
  int value;
};

template<class T>
int select(T &, char)
{
  return 1;
}

template<class T>
int select(T &sink, const char *)
{
  return sink.value == 5 ? 0 : 2;
}

typedef int (*Target)(Sink &, const char *);

int call(Target target)
{
  Sink sink;
  sink.value = 5;
  return target(sink, "x");
}

int main()
{
  return call(select);
}
