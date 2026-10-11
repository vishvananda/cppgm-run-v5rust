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

int main()
{
  Value local(22);
  Value result = true ? local : Value(33);
  (void)result;
  return local.payload == 22 && copies != 0 ? 0 : 1;
}
