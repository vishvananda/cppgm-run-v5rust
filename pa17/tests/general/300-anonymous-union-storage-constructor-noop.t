struct Value
{
  int x;

  Value(int v)
      : x(v)
  {
  }

  Value(const Value & other)
      : x(other.x)
  {
  }
};

template<class T>
struct Node
{
  union
  {
    T value;
  };

  template<class Arg>
  explicit Node(Arg &)
  {
  }

  Node(const Node &) = delete;
};

int main()
{
  Value value(1);
  Node<Value> node(value);
  (void)node;
  return 0;
}
