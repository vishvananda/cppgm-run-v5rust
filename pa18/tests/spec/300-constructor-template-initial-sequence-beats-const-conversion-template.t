// VALIDATION: compile-pass
// N3485 focus: 13.3.1.4 [over.match.copy], 13.3.3 [over.match.best]
// A converting constructor whose first parameter binds directly to the source
// beats a const conversion-function template when their result sequences tie.

struct external_code
{
  static const int bias = 10;
  int value;
};

struct error_id
{
  static const int bias = 0;
  int value;

  template<class T, class = typename T::accepts_external_code>
  operator T() const
  {
    return external_code{value};
  }
};

struct payload
{
};

template<class T>
struct result
{
  typedef int accepts_external_code;

  template<class A>
  result(A && error)
    : stored(error.value + A::bias)
  {
  }

  int stored;
};

result<payload> make_error()
{
  return error_id{7};
}

int main()
{
  result<payload> value = make_error();
  return value.stored == 7 ? 0 : 1;
}
