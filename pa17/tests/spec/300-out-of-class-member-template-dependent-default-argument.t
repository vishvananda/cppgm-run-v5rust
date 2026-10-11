// VALIDATION: compile-pass
// N3485 focus: 14.5.2 [temp.mem]

template<class T>
struct allocator
{
};

template<class T, class Allocator = allocator<T> >
struct sequence
{
  T value;
};

sequence<int> complete_sequence;

template<class Outer>
struct graph
{
  template<class Source>
  graph(sequence<Source> &);

  template<class Source>
  int accept(sequence<Source> &);
};

template<class Outer>
template<class Source>
graph<Outer>::graph(sequence<Source> &)
{
}

template<class Outer>
template<class Source>
int graph<Outer>::accept(sequence<Source> &)
{
  return sizeof(Source);
}

int main()
{
  sequence<int> source;
  graph<int> value(source);
  return value.accept(source) == static_cast<int>(sizeof(int)) ? 0 : 1;
}
