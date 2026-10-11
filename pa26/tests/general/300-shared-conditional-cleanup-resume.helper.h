struct Value
{
  int value;
  Value(int);
  Value(const Value &);
  ~Value() noexcept;
};

Value operator+(const Value &, const Value &);
extern int attempts, constructed, destroyed, live, fail_at;
