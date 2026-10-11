// N3485 focus: 5.14 [expr.log.and], 5.15 [expr.log.or],
// 12.3.2 [class.conv.fct], 14.5.2 [temp.mem]
// Contextual bool conversion for a built-in logical operator must prefer the
// non-template conversion function without instantiating a worse conversion
// function template body.

struct explicit_bool_source
{
  explicit operator bool() const { return true; }
};

struct value
{
  explicit operator bool() const { return true; }

  template<class T>
  operator T() const
  {
    return explicit_bool_source();
  }
};

int main()
{
  return value() && true && (value() || false) ? 0 : 1;
}
