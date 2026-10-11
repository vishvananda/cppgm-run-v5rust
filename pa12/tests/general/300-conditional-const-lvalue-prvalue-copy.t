int copies;

struct Value
{
  int payload;

  explicit Value(int input) : payload(input) {}

  Value(const Value& other) : payload(other.payload)
  {
    ++copies;
  }

  Value(Value&& other) : payload(other.payload)
  {
    other.payload = -1;
  }
};

Value choose(bool use_local)
{
  const Value local(22);
  return use_local ? local : Value(33);
}

int main()
{
  Value result = choose(true);
  return result.payload == 22 && copies != 0 ? 0 : 1;
}
