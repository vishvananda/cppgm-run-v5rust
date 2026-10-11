// VALIDATION: compile-pass
// N3485 focus: 13.3.1.4 [over.match.copy], 13.3.3 [over.match.best]
// When copy-initialization considers both a non-template converting
// constructor and a conversion-function template specialization, the
// non-template candidate wins after indistinguishable conversion sequences.

struct external_code
{
  int value;
};

struct error_id
{
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

  result(error_id error)
    : stored(error.value)
  {
  }

  result(external_code const & error)
    : stored(-error.value)
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
