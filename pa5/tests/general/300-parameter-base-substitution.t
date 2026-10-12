struct base { typedef int word; };
struct other { int word; };
template<class T> struct inherit : T {};
template<class... T> struct inherit_pack : T... {};
template<int N, class T, class U> struct second_parameter : U, T {};
using inherited = inherit<base>;
typedef inherited inherited_again;
using nested = inherit<inherited_again>;

void syntax() {
  inherit<base>::word * pointer;
  inherited::word * alias_pointer;
  inherited_again::word * typedef_pointer;
  nested::word * nested_pointer;
  inherit_pack<base, other>::word * first_is_type;
  inherit_pack<other, base>::word * first_is_value;
  second_parameter<0, base, other>::word * ordinary_bases_first;
  auto body = [] { inherit<base>::word * lambda_pointer; };
}
