// VALIDATION: compile-pass
// N3485 focus: 14.7.3 [temp.expl.spec], 14.5.2 [temp.mem]

namespace lib
{
struct date
{
  explicit date(int v_in) : v(v_in) {}
  int v;
};

enum special_value { min_date_time = 7 };
}

namespace traits
{
template<class T>
struct identity_element
{
  static T value();
};

template<class T>
inline T identity_element<T>::value()
{
  return T(1);
}

template<>
inline lib::date identity_element<lib::date>::value()
{
  return lib::date(lib::min_date_time);
}
}

int main()
{
  lib::date d = traits::identity_element<lib::date>::value();
  return d.v == 7 ? 0 : 1;
}
