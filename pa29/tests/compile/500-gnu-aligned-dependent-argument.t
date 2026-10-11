// The GNU aligned attribute takes a constant expression, not just a literal.
// Inside a class template the argument can name the enclosing template's own
// parameters, so it has no value until instantiation; the alignment is
// deferred to instantiation rather than diagnosed there, and instantiation
// must then actually apply it.

namespace probe
{

template<class T>
struct alignment_of
{
  static const unsigned long value = __alignof__(T);
};

template<class T>
const unsigned long alignment_v = __alignof__(T);

template<class T>
const unsigned long alignment_v<T&> = __alignof__(void*);

}

struct __attribute__((__aligned__(probe::alignment_of<double>::value))) named {};
static_assert(__alignof__(named) == __alignof__(double), "");

template<class T, class D>
struct holder
{
  typedef T* pointer;
  typedef D deleter_type;

  // Both spellings of a dependent argument: through a class template's static
  // member, and through a variable template.
  __attribute__((__aligned__(probe::alignment_of<deleter_type>::value)))
    pointer first_;
  __attribute__((__aligned__(probe::alignment_v<deleter_type>))) char second_;
};

static_assert(__alignof__(holder<int, double>) == __alignof__(double), "");
static_assert(__alignof__(holder<int, char>) == __alignof__(int*), "");

// A reference deleter selects the partial specialization, so the deferred
// argument has to be re-evaluated per specialization rather than cached.
static_assert(__alignof__(holder<int, double&>) == __alignof__(void*), "");
