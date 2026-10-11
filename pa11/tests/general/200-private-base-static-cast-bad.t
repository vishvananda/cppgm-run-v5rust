// VALIDATION: compile-fail
// A derived-to-base static_cast must honor the inheritance path's access.

struct base {};
class derived : base {};

base const * convert(derived const * value)
{
  return static_cast<base const *>(value);
}
