// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], 3.4.2 [basic.lookup.argdep],
// and 14.5.4 [temp.friend]. A detected-idiom decltype probe must find a
// hidden friend function template by ADL through cv-qualified pointer arguments.

struct false_type
{
  static const bool value = false;
};

struct true_type
{
  static const bool value = true;
};

template<class T>
T&& declval();

struct tag
{};

struct provider
{};

struct hash
{};

struct flavor
{};

template<class T, class En = void>
struct has_tag_invoke : false_type
{};

template<class T>
struct has_tag_invoke<T, decltype(
    tag_invoke(declval<tag const&>(),
               declval<provider&>(),
               declval<hash&>(),
               declval<flavor const&>(),
               declval<T const*>()),
    void())> : true_type
{};

class value
{
  int first;
  int second;

  template<class Provider, class Hash, class Flavor>
  friend void tag_invoke(tag const&, Provider const&, Hash&, Flavor const&,
                         value const*)
  {}
};

static_assert(has_tag_invoke<value>::value,
              "hidden friend function template should be found through pointer ADL");
static_assert(has_tag_invoke<value const>::value,
              "hidden friend function template should be found through cv pointer ADL");

int main()
{
  return 0;
}
