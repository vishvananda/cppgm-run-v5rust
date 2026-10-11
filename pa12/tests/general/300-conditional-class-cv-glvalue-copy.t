int copies;
int moves;

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
    ++moves;
    other.payload = -1;
  }
};

Value choose(bool use_local, const Value& fallback)
{
  Value local(11);
  return use_local ? local : fallback;
}

int main()
{
  Value fallback(22);
  Value result = choose(true, fallback);
  return result.payload == 11 && copies != 0 && moves == 0 &&
                 fallback.payload == 22
             ? 0
             : 1;
}
