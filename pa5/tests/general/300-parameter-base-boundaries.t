struct base { typedef int word; };
template<class T> struct inherit : T { void syntax() { word * not_substituted; } };
template<class... T> struct unexpanded : T {};
template<class T> struct no_parameters : T... {};
template<class T> struct inherit_twice : T, T {};
using pointer = inherit<base*>;

void syntax() {
  inherit<base*>::word * pointer_is_not_a_class;
  inherit<const base>::word * qualified_is_not_a_lone_name;
  inherit<base&>::word * reference_is_not_a_class;
  unexpanded<base>::word * pack_must_match;
  no_parameters<base>::word * nonpack_must_match;
  inherit_twice<base>::word * duplicate_base;
}
