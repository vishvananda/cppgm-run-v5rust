// VALIDATION: compile-pass

struct problem {};
struct callable {};

struct suite
{
  template<class F, class String>
  int except(F&& f, String const& reason);

  template<class F>
  int except(F&& f)
  {
    return except(f, "");
  }

  template<class E, class F, class String>
  int except(F&& f, String const& reason);

  template<class E, class F>
  int except(F&& f)
  {
    return except<E>(f, "");
  }
};

template<class F, class String>
int suite::except(F&& f, String const& reason)
{
  (void)f;
  return reason[0];
}

template<class E, class F, class String>
int suite::except(F&& f, String const& reason)
{
  (void)f;
  return reason[0];
}

int main()
{
  suite value;
  return value.except<problem>(callable());
}
