// VALIDATION: compile-pass
// N3485 focus: 14.5.7 [temp.alias], 14.8.2.1 [temp.deduct.call]

template<class T>
using direct_t = T;

struct value_type
{
};

template<class T>
int select(direct_t<T>)
{
  return 0;
}

int select(...)
{
  return 1;
}

int main()
{
  // A direct alias target remains a deduced context. Treating every alias as
  // the identity-style non-deduced case would select the fallback overload.
  value_type value;
  return select(value);
}
