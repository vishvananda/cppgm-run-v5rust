struct Value
{
  static int live;
  int * pointer;

  Value() : pointer(new int(7))
  {
    ++live;
  }

  Value(const Value & other) : pointer(new int(*other.pointer))
  {
    ++live;
  }

  ~Value()
  {
    delete pointer;
    --live;
  }
};

int Value::live = 0;

int consume(Value value)
{
  return *value.pointer;
}

int main()
{
  Value source;
  const int result = consume(source);
  return result == 7 && *source.pointer == 7 && Value::live == 1 ? 0 : 1;
}
