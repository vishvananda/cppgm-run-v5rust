// N3485 12.8p32: a selected deleted move does not retry the copy; expect reject
struct Value
{
  int n;
  Value();
  Value(const Value&);
  Value(Value&&) = delete;
};
Value result(Value value) { return value; }
