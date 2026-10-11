struct Value
{
  Value();
  Value(const Value &);
  ~Value() noexcept;
};

Value make_value();

Value choose()
{
  Value left;
  Value right;
  return make_value();
}
