struct Value
{
  Value() {}
  Value(const Value &) = delete;
  Value(Value &&) = default;
};

int main()
{
  Value first;
  Value second;
  Value copied = false ? first : second;
}
